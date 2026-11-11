//! First-mismatch comparison between two observation captures.
//!
//! The walk is frame-ordered: the earliest frame at which any observable
//! differs is the answer, regardless of which artifact carries it. Within
//! a frame, screen hash, then sound latch, then RAM CRCs, then checkpoint
//! pixels, then raw RAM dumps.

use std::collections::{BTreeSet, HashMap};
use std::fmt;

use crate::capture::Capture;
use crate::scenario::Scenario;

/// The frame-observable columns a scoped comparison may check.
pub const FRAME_OBSERVABLES: [&str; 5] = [
    "screen_hash",
    "latch",
    "latch_changed",
    "ram_crc",
    "music_crc",
];

/// Optional restriction of the comparison walk: an inclusive frame range
/// and/or a subset of the frame-observable columns. Checkpoint pixels and
/// RAM dumps are compared whenever both sides carry them for an in-range
/// frame; the field scope applies to the per-frame columns only.
///
/// A scoped comparison is how a bounded slice states, in machine-checkable
/// form, which observables it claims to cover: everything outside the scope
/// is explicitly not validated by that run.
#[derive(Debug, Clone, Default)]
pub struct CompareScope {
    pub from: Option<u64>,
    pub to: Option<u64>,
    pub fields: Option<BTreeSet<String>>,
}

impl CompareScope {
    /// The unrestricted walk: all frames, all observables.
    pub fn full() -> Self {
        CompareScope::default()
    }

    pub fn with_range(mut self, from: u64, to: u64) -> Self {
        self.from = Some(from);
        self.to = Some(to);
        self
    }

    pub fn with_fields(mut self, fields: BTreeSet<String>) -> Self {
        self.fields = Some(fields);
        self
    }

    /// Parse a comma-separated field list, rejecting unknown names.
    pub fn parse_fields(spec: &str) -> Result<BTreeSet<String>, String> {
        let mut fields = BTreeSet::new();
        for name in spec.split(',') {
            let name = name.trim();
            if !FRAME_OBSERVABLES.contains(&name) {
                return Err(format!(
                    "unknown frame observable '{name}' (known: {})",
                    FRAME_OBSERVABLES.join(", ")
                ));
            }
            fields.insert(name.to_string());
        }
        Ok(fields)
    }

    /// Human-readable statement of the restriction, for reports.
    pub fn describe(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.from.is_some() || self.to.is_some() {
            parts.push(format!(
                "frames {}..={}",
                self.from.map(|f| f.to_string()).unwrap_or("0".into()),
                self.to.map(|f| f.to_string()).unwrap_or("end".into())
            ));
        }
        if let Some(fields) = &self.fields {
            parts.push(format!(
                "frame observables limited to: {}",
                fields.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        (!parts.is_empty()).then(|| parts.join("; "))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    /// The candidate capture was produced by the pinned MAME baseline:
    /// a reference-versus-reference self-check.
    Reference,
    /// The candidate capture was produced by the native port.
    Native,
}

#[derive(Debug)]
pub enum Outcome {
    Match {
        frames_compared: u64,
        checkpoints_compared: usize,
    },
    Mismatch(Mismatch),
}

#[derive(Debug)]
pub struct Mismatch {
    pub frame: u64,
    pub observable: String,
    pub expected: String,
    pub actual: String,
    /// Extra locating detail, e.g. the first differing pixel coordinate.
    pub detail: Option<String>,
}

#[derive(Debug)]
pub enum CompareError {
    Incompatible(String),
    Io(String),
}

impl fmt::Display for CompareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareError::Incompatible(msg) => write!(f, "incompatible comparison inputs: {msg}"),
            CompareError::Io(msg) => write!(f, "comparison I/O failure: {msg}"),
        }
    }
}

impl std::error::Error for CompareError {}

pub fn compare(
    scenario: &Scenario,
    reference: &Capture,
    candidate: &Capture,
    candidate_kind: CandidateKind,
    scope: &CompareScope,
) -> Result<Outcome, CompareError> {
    check_identity(scenario, reference, candidate, candidate_kind)?;

    let ref_frames: HashMap<u64, _> = reference.frames.iter().map(|f| (f.frame, f)).collect();
    let cand_frames: HashMap<u64, _> = candidate.frames.iter().map(|f| (f.frame, f)).collect();
    let ref_checkpoints: HashMap<u64, _> =
        reference.checkpoints.iter().map(|c| (c.frame, c)).collect();
    let cand_checkpoints: HashMap<u64, _> =
        candidate.checkpoints.iter().map(|c| (c.frame, c)).collect();
    let ref_dumps = dumps_by_frame(reference);
    let cand_dumps = dumps_by_frame(candidate);

    let duration = scenario
        .duration_frames
        .max(reference.frames.last().map(|f| f.frame + 1).unwrap_or(0));
    let from = scope.from.unwrap_or(0);
    let to = scope.to.unwrap_or_else(|| duration.saturating_sub(1));
    if from > to {
        return Err(CompareError::Incompatible(format!(
            "invalid comparison frame range {from}..={to}"
        )));
    }
    let mut frames_compared = 0;
    let mut checkpoints_compared = 0;

    for frame in from..=to {
        match (ref_frames.get(&frame), cand_frames.get(&frame)) {
            (Some(r), Some(c)) => {
                if let Some(m) = compare_frame_observations(r, c, scope.fields.as_ref()) {
                    return Ok(Outcome::Mismatch(m));
                }
                frames_compared += 1;
            }
            (Some(_), None) => {
                return Ok(Outcome::Mismatch(Mismatch {
                    frame,
                    observable: "frame observation".into(),
                    expected: "present".into(),
                    actual: format!("candidate capture ended before frame {frame}"),
                    detail: None,
                }));
            }
            (None, Some(_)) => {
                return Ok(Outcome::Mismatch(Mismatch {
                    frame,
                    observable: "frame observation".into(),
                    expected: format!("reference capture ended before frame {frame}"),
                    actual: "present".into(),
                    detail: None,
                }));
            }
            (None, None) => {}
        }

        if let (Some(r), Some(c)) = (ref_checkpoints.get(&frame), cand_checkpoints.get(&frame)) {
            if r.screen_crc32 != c.screen_crc32 {
                let detail = pixel_diff_detail(r, c, reference.meta.screen_width)?;
                return Ok(Outcome::Mismatch(Mismatch {
                    frame,
                    observable: "checkpoint screen pixels".into(),
                    expected: format!("crc32 {}", r.screen_crc32),
                    actual: format!("crc32 {}", c.screen_crc32),
                    detail,
                }));
            }
            checkpoints_compared += 1;
        }

        for (region, r_dump) in ref_dumps.get(&frame).into_iter().flatten() {
            let Some(c_dump) = cand_dumps
                .get(&frame)
                .and_then(|regions| regions.get(region))
            else {
                continue;
            };
            if let Some(m) = compare_ram_dumps(frame, region, r_dump, c_dump)? {
                return Ok(Outcome::Mismatch(m));
            }
        }
    }

    Ok(Outcome::Match {
        frames_compared,
        checkpoints_compared,
    })
}

fn check_identity(
    scenario: &Scenario,
    reference: &Capture,
    candidate: &Capture,
    candidate_kind: CandidateKind,
) -> Result<(), CompareError> {
    for (label, capture) in [("reference", reference), ("candidate", candidate)] {
        match &capture.scenario_id {
            Some(id) if *id == scenario.scenario_id => {}
            Some(id) => {
                return Err(CompareError::Incompatible(format!(
                    "{label} capture {} is of scenario '{id}', not '{}'",
                    capture.dir.display(),
                    scenario.scenario_id
                )))
            }
            None => {
                return Err(CompareError::Incompatible(format!(
                    "{label} capture {} lacks identity (summary.json); cannot establish which scenario it observes",
                    capture.dir.display()
                )))
            }
        }
        if let Some(dsw1) = capture.meta.dsw1 {
            if dsw1 != scenario.dsw1 {
                return Err(CompareError::Incompatible(format!(
                    "{label} capture {} readback DSW1=0x{dsw1:04x} does not match scenario's 0x{:04x}",
                    capture.dir.display(),
                    scenario.dsw1
                )));
            }
        }
        if let Some(dsw2) = capture.meta.dsw2 {
            if dsw2 != scenario.dsw2 {
                return Err(CompareError::Incompatible(format!(
                    "{label} capture {} readback DSW2=0x{dsw2:04x} does not match scenario's 0x{:04x}",
                    capture.dir.display(),
                    scenario.dsw2
                )));
            }
        }
    }
    match &reference.mame_version_reported {
        Some(v) if v.starts_with(&scenario.reference_mame_version) => {}
        Some(v) => {
            return Err(CompareError::Incompatible(format!(
                "reference capture was produced by MAME {v}, but the scenario pins {}",
                scenario.reference_mame_version
            )))
        }
        None => {
            return Err(CompareError::Incompatible(format!(
                "reference capture {} lacks a reported MAME version; baseline identity cannot be established",
                reference.dir.display()
            )))
        }
    }
    if candidate_kind == CandidateKind::Reference {
        if let Some(v) = &candidate.mame_version_reported {
            if !v.starts_with(&scenario.reference_mame_version) {
                return Err(CompareError::Incompatible(format!(
                    "candidate capture was produced by MAME {v}, but the scenario pins {}",
                    scenario.reference_mame_version
                )));
            }
        }
    }
    if (reference.meta.screen_width, reference.meta.screen_height)
        != (candidate.meta.screen_width, candidate.meta.screen_height)
    {
        return Err(CompareError::Incompatible(format!(
            "screen geometry differs: reference {}x{}, candidate {}x{}",
            reference.meta.screen_width,
            reference.meta.screen_height,
            candidate.meta.screen_width,
            candidate.meta.screen_height
        )));
    }
    Ok(())
}

fn compare_frame_observations(
    r: &crate::capture::FrameObservation,
    c: &crate::capture::FrameObservation,
    scope_fields: Option<&BTreeSet<String>>,
) -> Option<Mismatch> {
    let observables = [
        ("screen_hash", &r.screen_hash, &c.screen_hash),
        ("latch", &r.latch, &c.latch),
        (
            "latch_changed",
            &(r.latch_changed as u8).to_string(),
            &(c.latch_changed as u8).to_string(),
        ),
        (
            "ram_crc",
            &r.ram_crc.clone().unwrap_or_else(|| "-".into()),
            &c.ram_crc.clone().unwrap_or_else(|| "-".into()),
        ),
        (
            "music_crc",
            &r.music_crc.clone().unwrap_or_else(|| "-".into()),
            &c.music_crc.clone().unwrap_or_else(|| "-".into()),
        ),
    ];
    for (name, expected, actual) in observables {
        if let Some(scope) = scope_fields {
            if !scope.contains(name) {
                continue;
            }
        }
        if expected != actual {
            return Some(Mismatch {
                frame: r.frame,
                observable: name.to_string(),
                expected: expected.clone(),
                actual: actual.clone(),
                detail: None,
            });
        }
    }
    None
}

fn read_bytes(path: &std::path::Path) -> Result<Vec<u8>, CompareError> {
    std::fs::read(path).map_err(|e| CompareError::Io(format!("{}: {e}", path.display())))
}

fn pixel_diff_detail(
    r: &crate::capture::Checkpoint,
    c: &crate::capture::Checkpoint,
    width: u32,
) -> Result<Option<String>, CompareError> {
    let (a, b) = (read_bytes(&r.pixels_path)?, read_bytes(&c.pixels_path)?);
    let width = width.max(1) as usize;
    let pixels = a.len() / 4;
    for i in 0..pixels.min(b.len() / 4) {
        let (pa, pb) = (&a[i * 4..i * 4 + 4], &b[i * 4..i * 4 + 4]);
        if pa != pb {
            // Buffers are B,G,R,A bytes; report ARGB as the game sees it.
            let argb = |p: &[u8]| format!("{:02x}{:02x}{:02x}{:02x}", p[3], p[2], p[1], p[0]);
            return Ok(Some(format!(
                "first differing pixel at ({}, {}): expected argb {}, actual {}",
                i % width,
                i / width,
                argb(pa),
                argb(pb)
            )));
        }
    }
    Ok(None)
}

fn dumps_by_frame(capture: &Capture) -> HashMap<u64, HashMap<String, std::path::PathBuf>> {
    let mut map: HashMap<u64, HashMap<String, std::path::PathBuf>> = HashMap::new();
    for dump in &capture.ram_dumps {
        map.entry(dump.frame)
            .or_default()
            .insert(dump.region.clone(), dump.path.clone());
    }
    map
}

fn compare_ram_dumps(
    frame: u64,
    region: &str,
    r_path: &std::path::Path,
    c_path: &std::path::Path,
) -> Result<Option<Mismatch>, CompareError> {
    let (a, b) = (read_bytes(r_path)?, read_bytes(c_path)?);
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i), b.get(i));
        if x != y {
            return Ok(Some(Mismatch {
                frame,
                observable: format!("ram dump '{region}'"),
                expected: x
                    .map(|v| format!("byte {v:02x}"))
                    .unwrap_or("absent".into()),
                actual: y
                    .map(|v| format!("byte {v:02x}"))
                    .unwrap_or("absent".into()),
                detail: Some(format!("first differing offset {i:#06x}")),
            }));
        }
    }
    Ok(None)
}
