//! Local ROM input-data validation against the committed manifest.
//!
//! The manifest (`tools/reference/rom-manifest.json`) records the expected
//! name, region, size, CRC32, and SHA1 of every entry in the reference ROM
//! set, transcribed from the pinned MAME source. Verification proves the
//! local game data is the verified set before any comparison trusts it.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub rom_set: String,
    #[serde(default)]
    pub entries: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ManifestEntry {
    pub name: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub offset: u64,
    pub size: u64,
    pub crc32: String,
    pub sha1: String,
}

#[derive(Debug)]
pub enum RomProblem {
    ManifestUnreadable {
        path: PathBuf,
        reason: String,
    },
    Missing {
        name: String,
    },
    WrongSize {
        name: String,
        expected: u64,
        actual: u64,
    },
    Crc32Mismatch {
        name: String,
        expected: String,
        actual: String,
    },
    Sha1Mismatch {
        name: String,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for RomProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RomProblem::ManifestUnreadable { path, reason } => {
                write!(f, "cannot read ROM manifest {}: {reason}", path.display())
            }
            RomProblem::Missing { name } => write!(f, "ROM entry {name} is missing"),
            RomProblem::WrongSize {
                name,
                expected,
                actual,
            } => write!(
                f,
                "ROM entry {name} has size {actual:#x}, expected {expected:#x}"
            ),
            RomProblem::Crc32Mismatch {
                name,
                expected,
                actual,
            } => write!(
                f,
                "ROM entry {name} crc32 mismatch: actual {actual}, expected {expected}"
            ),
            RomProblem::Sha1Mismatch {
                name,
                expected,
                actual,
            } => write!(
                f,
                "ROM entry {name} sha1 mismatch: actual {actual}, expected {expected}"
            ),
        }
    }
}

/// Error type returned by [`verify_roms`]: one or more problems with the
/// local input data. Display lists every problem found.
#[derive(Debug)]
pub struct RomVerificationError(pub Vec<RomProblem>);

impl fmt::Display for RomVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "local ROM data failed verification:")?;
        for problem in &self.0 {
            writeln!(f, "  - {problem}")?;
        }
        Ok(())
    }
}

impl std::error::Error for RomVerificationError {}

pub fn load_manifest(path: &Path) -> Result<Manifest, RomVerificationError> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        RomVerificationError(vec![RomProblem::ManifestUnreadable {
            path: path.to_path_buf(),
            reason: e.to_string(),
        }])
    })?;
    serde_json::from_str(&text).map_err(|e| {
        RomVerificationError(vec![RomProblem::ManifestUnreadable {
            path: path.to_path_buf(),
            reason: e.to_string(),
        }])
    })
}

/// Verify every manifest entry against the files in `rom_dir`.
/// Extra files in the directory are ignored, as the baseline ignores them.
pub fn verify_roms(
    manifest_path: impl AsRef<Path>,
    rom_dir: impl AsRef<Path>,
) -> Result<(), RomVerificationError> {
    let manifest = load_manifest(manifest_path.as_ref())?;
    let rom_dir = rom_dir.as_ref();
    let mut problems = Vec::new();

    for entry in &manifest.entries {
        let path = rom_dir.join(&entry.name);
        let Ok(bytes) = std::fs::read(&path) else {
            problems.push(RomProblem::Missing {
                name: entry.name.clone(),
            });
            continue;
        };
        if bytes.len() as u64 != entry.size {
            problems.push(RomProblem::WrongSize {
                name: entry.name.clone(),
                expected: entry.size,
                actual: bytes.len() as u64,
            });
            continue;
        }
        let actual_crc = format!("{:08x}", crc32fast::hash(&bytes));
        if actual_crc != entry.crc32.to_lowercase() {
            problems.push(RomProblem::Crc32Mismatch {
                name: entry.name.clone(),
                expected: entry.crc32.clone(),
                actual: actual_crc,
            });
            continue;
        }
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let actual_sha1: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if actual_sha1 != entry.sha1.to_lowercase() {
            problems.push(RomProblem::Sha1Mismatch {
                name: entry.name.clone(),
                expected: entry.sha1.clone(),
                actual: actual_sha1,
            });
        }
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(RomVerificationError(problems))
    }
}
