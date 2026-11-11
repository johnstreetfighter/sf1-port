//! Tickets #144 (F72) & #149 (F77): Full-game completeness and fidelity audit
//! and test-mode audit addendum.
//!
//! Evidence: docs/reference/full-game-audit.md and docs/reference/coverage-ledger.md.

use std::path::Path;

const AUDIT_MD: &str = "../../docs/reference/full-game-audit.md";
const LEDGER_MD: &str = "../../docs/reference/coverage-ledger.md";
const LEDGER_JSON: &str = "../../tools/reference/local/coverage/ledger.json";

#[test]
fn full_game_audit_covers_all_user_stories_decisions_mame_behaviours_and_non_certifications() {
    let audit_path = Path::new(AUDIT_MD);
    assert!(audit_path.is_file(), "missing {}", audit_path.display());
    let text = std::fs::read_to_string(audit_path).expect("read full-game-audit.md");

    // 1. All 34 user stories (US-1..US-34) present.
    for us in 1..=34 {
        let tag = format!("| **US-{us}** |");
        assert!(
            text.contains(&tag),
            "full-game-audit.md missing user story row: {tag}"
        );
    }

    // 2. All 13 testing decisions (1..13) present.
    for dec in 1..=13 {
        let tag = format!("| {dec} |");
        assert!(
            text.contains(&tag),
            "full-game-audit.md missing testing decision row: {tag}"
        );
    }

    // 3. Required sections, dispositions, MAME behaviours (including F73/F77 Row 9),
    // person checks, non-certification statement, and F77 (#149) test-mode addendum.
    for required in [
        "## 1. End-to-end full-game baseline before test mode",
        "## 2. Pass/fail account against the parent specification (`#1`)",
        "### 2.1 User stories (`US-1..US-34`)",
        "### 2.2 Testing decisions in the parent specification (`#1`)",
        "## 3. Final pre-test-mode coverage ledger (`F11` / `#51`) and dispositions",
        "### 3.1 Final pre-test-mode coverage ledger run",
        "### 3.2 Disposition of every `note()` and `note_unrecovered()` report site in the ledger",
        "### 3.3 Disposition of every `\"Not covered\"` item across `docs/reference/*.md`",
        "`service-sound-test.md` (`F22` / `#62`), `music-tunes.md` (`F23` / `#63`), `sound-test-03.md` / `adpcm-samples.md` (`F38` / `#78`)",
        "`round-outcomes.md` (`F32` / `#72`)",
        "## 4. Adopted MAME `0.289` behaviours and their source locations",
        "**The 500-ns scheduler slice**",
        "**The NMI at a slice start**",
        "**The interrupt-latency band (`45–138` 68000 cycles)**",
        "**The output resampler**",
        "**The coin lockout**",
        "sf-mame0289.cpp:110–116",
        "**The power-on RAM contents (`F1`)**",
        "**The fixed `IPT_UNKNOWN` `SYSTEM` bits**",
        "sf-mame0289.cpp:558–570",
        "**The vblank held (`irq1_line_hold`) across the reset's RAM clear**",
        "sf-mame0289.cpp:758",
        "**The unmapped-port read value (`unmap_value_high` = `0xffff` on `0xc00004`, `0xc00006`, `0xc0000e`)**",
        "sf-mame0289.cpp:167, 173–174, 178",
        "## 5. Citations and results of the three person checks (`F30`, `F31`, `F54`)",
        "Ticket [#70]",
        "Ticket [#71]",
        "Ticket [#126]",
        "## 6. Explicit non-certification statement",
        "## 7. Addendum: Test-mode result and final batch closure (`F77` / `#149`)",
        "### 7.1 Results of `F73–F76` (`#145–#148`)",
        "`sf-service-01`",
        "`sf-service-02`",
        "`sf-service-03a`",
        "`sf-service-03b`",
        "### 7.2 Coverage ledger rerun over `0x4834e..0x492d5` and its helpers (Acceptance 1)",
        "| **Total** | **`162` (`161` executed, `1` unexecuted)** | **`10,916`** | **`10,916` (`100.0%`)** | **`0`** | **`0`** |",
        "### 7.3 Unmapped-port value (`F73` / `#145`) added to adopted MAME `0.289` behaviours",
        "### 7.4 Every open decision and every deferred item with its outcome (Acceptance 2)",
        "#### 7.4.1 Outcome of every open decision in `#1` (Specification) (`1..8`)",
        "#### 7.4.2 Outcome of every open decision in `#13` / `docs/planning/next-batch.md` (`1..4`)",
        "#### 7.4.3 Outcome of every open decision in `#40` / `docs/planning/arcade-flow-batch.md` §7 (`1..16`)",
        "#### 7.4.4 Complete list of deferred / excluded items and their outcomes",
        "### 7.5 Final pass / deferred / fail account and certification boundary (Acceptance 3)",
        "#### Explicit certification boundary (`F77` / `#149`)",
        "Nothing that does not pass is certified.",
    ] {
        assert!(
            text.contains(required),
            "full-game-audit.md missing required section or citation: {required}"
        );
    }

    let ledger_md_path = Path::new(LEDGER_MD);
    assert!(
        ledger_md_path.is_file(),
        "missing {}",
        ledger_md_path.display()
    );
    let ledger_md = std::fs::read_to_string(ledger_md_path).expect("read coverage-ledger.md");
    assert!(
        ledger_md.contains(
            "**Normal-play (`Service Mode = Off`, 128 scenarios, `F1–F71` pre-test-mode scope):** `9990` executed PCs (`8389` `:maincpu`, `1378` `:audiocpu`, `223` `:audio2`), `9990` translated, **`0` untranslated**."
        ),
        "coverage-ledger.md missing zero-untranslated normal-play summary line"
    );
    assert!(
        ledger_md.contains("| **Total** | | **10916** | **10916** | **0** | **0** |"),
        "coverage-ledger.md missing 10916 / 10916 zero-untranslated total row across all 137 scenarios"
    );
}

#[test]
fn local_ledger_json_confirms_zero_untranslated_and_complete_note_dispositions() {
    let json_path = Path::new(LEDGER_JSON);
    if !json_path.is_file() {
        eprintln!("skipping: local coverage ledger.json not present");
        return;
    }

    let raw = std::fs::read_to_string(json_path).expect("read ledger.json");
    let ledger: serde_json::Value = serde_json::from_str(&raw).expect("parse ledger.json");

    assert_eq!(
        ledger["scenario_count"].as_u64().expect("scenario_count"),
        137,
        "expected 137 scenarios in coverage ledger"
    );

    let cpu_totals = &ledger["cpu_totals"];
    for (cpu, expected_exec) in [("maincpu", 9299u64), ("audiocpu", 1394), ("audio2", 223)] {
        let executed = cpu_totals[cpu]["executed_pcs"]
            .as_u64()
            .expect("executed_pcs u64");
        let translated = cpu_totals[cpu]["translated_pcs"]
            .as_u64()
            .expect("translated_pcs u64");
        let untranslated = cpu_totals[cpu]["untranslated_pcs"]
            .as_u64()
            .expect("untranslated_pcs u64");
        assert_eq!(executed, expected_exec, "all-scenario {cpu} executed_pcs");
        assert_eq!(
            translated, expected_exec,
            "all-scenario {cpu} translated_pcs"
        );
        assert_eq!(
            untranslated, 0,
            "all-scenario {cpu} has {untranslated} untranslated PCs (expected 0)"
        );
    }

    let normal_totals = &ledger["normal_play_cpu_totals"];
    for (cpu, expected_exec) in [("maincpu", 8389u64), ("audiocpu", 1378), ("audio2", 223)] {
        let executed = normal_totals[cpu]["executed_pcs"]
            .as_u64()
            .expect("executed_pcs u64");
        let translated = normal_totals[cpu]["translated_pcs"]
            .as_u64()
            .expect("translated_pcs u64");
        let untranslated = normal_totals[cpu]["untranslated_pcs"]
            .as_u64()
            .expect("untranslated_pcs u64");
        assert_eq!(executed, expected_exec, "normal-play {cpu} executed_pcs");
        assert_eq!(
            translated, expected_exec,
            "normal-play {cpu} translated_pcs"
        );
        assert_eq!(
            untranslated, 0,
            "normal-play {cpu} has {untranslated} untranslated PCs (expected 0)"
        );
    }

    let doc_gaps = ledger["documented_gaps"]
        .as_array()
        .expect("documented_gaps array");
    assert!(
        doc_gaps.is_empty(),
        "expected 0 documented_gaps across all 137 scenarios, got {}",
        doc_gaps.len()
    );

    let undoc_all = ledger["undocumented_untranslated"]
        .as_array()
        .expect("undocumented_untranslated array");
    assert!(
        undoc_all.is_empty(),
        "expected 0 undocumented_untranslated across all 137 scenarios, got {}",
        undoc_all.len()
    );

    let note_sites = ledger["note_sites"].as_array().expect("note_sites array");
    assert_eq!(note_sites.len(), 27, "25 original note sites plus 2 extension-only raw-score guards");
    for site in note_sites {
        let file = site["file"].as_str().unwrap_or("?");
        let line = site["line"].as_u64().unwrap_or(0);
        let disp = site["disposition"].as_str().unwrap_or("");
        assert!(
            !disp.is_empty(),
            "note site {file}:{line} is missing a disposition"
        );
        let reached = site["reached"].as_bool().expect("reached bool");
        assert!(
            !reached,
            "note site {file}:{line} is unexpectedly reached after F73–F76: {disp}"
        );
        assert!(
            disp.starts_with("Unreachable under MAME") || disp.starts_with("Deferred"),
            "unreached note site {file}:{line} has unexpected disposition: {disp}"
        );
    }
}
