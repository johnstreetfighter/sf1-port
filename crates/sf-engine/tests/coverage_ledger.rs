//! Ticket #51 (F11): Translation coverage ledger from every pinned reference.
//!
//! Evidence: docs/reference/coverage-ledger.md.

use std::path::Path;
use std::process::Command;

const LEDGER_MD: &str = "../../docs/reference/coverage-ledger.md";
const LEDGER_SCRIPT: &str = "../../tools/reference/coverage_ledger.py";
const COVERAGE_PROBE_SAMPLE: &str =
    "../../tools/reference/local/coverage/sf-fixed-fight-01-run-a.tsv";

#[test]
fn tracked_coverage_ledger_contains_required_sections_and_zero_unnamed_gaps() {
    let md_path = Path::new(LEDGER_MD);
    assert!(md_path.is_file(), "missing {}", md_path.display());
    let text = std::fs::read_to_string(md_path).expect("read coverage-ledger.md");

    for required in [
        "## 1. Probe reproducibility and summary",
        "### 1.1 Per-scenario probe runs (`run-a == run-b`)",
        "## 2. Executed original addresses mapped to the Rust that cites them",
        "### 2.1 `:maincpu` (68000)",
        "### 2.2 `:audiocpu` (music Z80)",
        "### 2.3 `:audio2` (samples Z80)",
        "## 3. Translated routines mapped to the scenarios that execute them",
        "## 4. `note()` and `note_unrecovered()` report sites",
        "## 5. Documented gaps reproduced on the tree of the day",
        "## 6. Executed, untranslated addresses that no doc names",
        "None (`0` addresses across `:maincpu`, `:audiocpu`, and `:audio2`).",
        "## 7. Verification and negative control (`C-NEG`)",
    ] {
        assert!(
            text.contains(required),
            "coverage-ledger.md missing expected section or verdict: {required}"
        );
    }
}

#[test]
fn coverage_ledger_one_command_check_and_negative_control() {
    if !Path::new(COVERAGE_PROBE_SAMPLE).is_file() {
        eprintln!("skipping: local coverage probe data not present");
        return;
    }

    let script = Path::new(LEDGER_SCRIPT);
    assert!(script.is_file(), "missing {}", script.display());

    let out = Command::new("python")
        .arg(script)
        .arg("--check")
        .output()
        .expect("invoke python tools/reference/coverage_ledger.py --check");
    assert!(
        out.status.success(),
        "coverage_ledger.py --check failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
