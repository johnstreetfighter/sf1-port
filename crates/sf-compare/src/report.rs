//! Readable comparison reports: the first mismatch, expected and actual
//! observations, baseline identity, and the scenario needed to reproduce.

use crate::capture::Capture;
use crate::compare::{CandidateKind, CompareScope, Mismatch, Outcome};
use crate::scenario::Scenario;

pub fn render(
    outcome: &Outcome,
    scenario: &Scenario,
    reference: &Capture,
    candidate: &Capture,
    candidate_kind: CandidateKind,
    scope: &CompareScope,
) -> String {
    let mut out = String::new();
    match outcome {
        Outcome::Match {
            frames_compared,
            checkpoints_compared,
        } => {
            out.push_str(&format!(
                "MATCH: {frames_compared} frames, {checkpoints_compared} checkpoints compared; no observable difference\n"
            ));
            if let Some(desc) = scope.describe() {
                out.push_str(&format!(
                    "  scope: {desc} — everything outside this scope is explicitly not validated by this run\n"
                ));
            }
            if candidate_kind == CandidateKind::Reference {
                out.push_str(
                    "  note: reference-versus-reference self-check (the candidate is a baseline capture, not native behavior)\n",
                );
            }
        }
        Outcome::Mismatch(m) => {
            out.push_str(&render_mismatch(m));
            if let Some(desc) = scope.describe() {
                out.push_str(&format!("  scope: {desc}\n"));
            }
            out.push_str(&format!("  scenario: {}\n", scenario.scenario_id));
            out.push_str(&format!(
                "  baseline: MAME {}, ROM set {} ({})\n",
                scenario.reference_mame_version,
                scenario.reference_rom_set,
                reference.dir.display()
            ));
            out.push_str(&format!(
                "  candidate: {} ({})\n",
                candidate.dir.display(),
                match candidate_kind {
                    CandidateKind::Reference =>
                        "reference capture — reference-versus-reference self-check",
                    CandidateKind::Native => "native capture",
                }
            ));
            let scenario_path = scenario
                .source_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<scenario.json>".into());
            out.push_str(&format!(
                "  reproduce reference: python tools/reference/run_scenario.py {scenario_path} --out <capture-dir>\n"
            ));
        }
    }
    out
}

fn render_mismatch(m: &Mismatch) -> String {
    let mut out = format!(
        "MISMATCH at frame {}\n  observable: {}\n  expected (reference): {}\n  actual   (candidate): {}\n",
        m.frame, m.observable, m.expected, m.actual
    );
    if let Some(detail) = &m.detail {
        out.push_str(&format!("  detail: {detail}\n"));
    }
    out
}
