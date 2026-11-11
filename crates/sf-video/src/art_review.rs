//! The user's review decisions for one fighter (spec #164, decisions 7, 8
//! and 10). Kept in a committed file, `art/review/<fighter>.json`, which the
//! game embeds: a fighter's new poses reach the game only once all of them
//! are approved (decision 9).
//! - Each pose in his catalogue's moves is approved or rejected.
//! - A rejected free pose (a ROM pose or a recipe) is marked for
//!   generation.
//! - A rejected generated pose is marked for another try. After three
//!   rejections it goes back to the user to decide.
//! - A verdict on a generated pose holds for the file it was given on (its
//!   CRC-32). A new file, such as a retry, is pending again, and keeps its
//!   rejections toward the three.
//! - The fighter is approved only when every pose is.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use crate::art::{Catalogue, Source};

/// Rejections of a generated pose before it goes back to the user.
pub const RETRY_CAP: u32 = 3;

/// Where a pose's art comes from, as review sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    /// From the ROM, used as it is or rebuilt from parts: free.
    Free,
    /// Generated, its pixels in the art pack.
    Generated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    Pending,
    Approved,
    Rejected,
}

/// What a pose needs next.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Next {
    /// Nothing: approved, or waiting for review.
    None,
    /// A rejected free pose: generate it instead.
    Generate,
    /// A rejected generated pose: try again.
    Retry,
    /// Rejected three times: the user decides (take the best attempt,
    /// simplify the move, or drop the pose).
    UserDecides,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoseReview {
    pub origin: Origin,
    /// The art pack file a generated pose was judged on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crc32: Option<String>,
    pub verdict: Verdict,
    pub rejections: u32,
    pub next: Next,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    pub fighter: String,
    pub poses: BTreeMap<String, PoseReview>,
}

impl Review {
    /// Every pose in the catalogue's moves, pending, keeping what `old`
    /// already decided for a pose whose origin is unchanged.
    pub fn for_catalogue(catalogue: &Catalogue, old: Option<&Review>) -> Review {
        let mut poses = BTreeMap::new();
        for m in &catalogue.moves {
            for name in &m.poses {
                let Some(pose) = catalogue.pose(name) else { continue };
                let origin = match pose.source {
                    Source::Rom | Source::Recipe => Origin::Free,
                    Source::Pack => Origin::Generated,
                };
                let kept = old.and_then(|r| r.poses.get(name)).filter(|p| p.origin == origin).cloned().map(|p| {
                    if p.crc32 == pose.pack {
                        p
                    } else {
                        PoseReview { crc32: pose.pack.clone(), verdict: Verdict::Pending, next: Next::None, ..p }
                    }
                });
                poses.entry(name.clone()).or_insert(kept.unwrap_or(PoseReview {
                    origin,
                    crc32: pose.pack.clone(),
                    verdict: Verdict::Pending,
                    rejections: 0,
                    next: Next::None,
                }));
            }
        }
        Review { fighter: catalogue.fighter.clone(), poses }
    }

    /// Record a verdict on `pose`.
    pub fn decide(&mut self, pose: &str, approve: bool) -> Result<&PoseReview, String> {
        let p = self.poses.get_mut(pose).ok_or_else(|| format!("{} has no pose named {pose}", self.fighter))?;
        if approve {
            p.verdict = Verdict::Approved;
            p.next = Next::None;
        } else {
            p.verdict = Verdict::Rejected;
            p.rejections += 1;
            p.next = match p.origin {
                Origin::Free => Next::Generate,
                Origin::Generated if p.rejections >= RETRY_CAP => Next::UserDecides,
                Origin::Generated => Next::Retry,
            };
        }
        Ok(p)
    }

    /// The fighter is approved when every pose is.
    pub fn approved(&self) -> bool {
        !self.poses.is_empty() && self.poses.values().all(|p| p.verdict == Verdict::Approved)
    }

    pub fn load(path: &Path) -> Option<Review> {
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())? + "\n";
        std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
    }
}
