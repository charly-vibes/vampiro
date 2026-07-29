//! Ecosystem dogfood-2 verification (vampiro-tmf.2).
//!
//! Lightweight structural checks over `docs/verification/dogfood-2.md`. The
//! full dogfood run (vampiro check across the charly-vibes suite) is a
//! human-in-the-loop exercise whose results are recorded in that document;
//! these tests assert the document exists and contains the required sections
//! so the dogfood record cannot silently regress or be deleted.
//!
//! Run: `cargo test -p vampiro --test dogfood_ecosystem_tests`

/// Resolve the workspace root from this crate's manifest dir.
fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn dogfood2_path() -> std::path::PathBuf {
    workspace_root()
        .join("docs")
        .join("verification")
        .join("dogfood-2.md")
}

fn read_doc() -> String {
    let path = dogfood2_path();
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn dogfood_ecosystem_doc_exists() {
    let path = dogfood2_path();
    assert!(
        path.exists(),
        "docs/verification/dogfood-2.md must exist (vampiro-tmf.2)"
    );
}

#[test]
fn dogfood_ecosystem_doc_has_required_sections() {
    let doc = read_doc();
    for section in [
        "# Dogfooding Run 2",
        "## Summary",
        "## Findings by class",
        "## Per-repo breakdown",
        "## Scan-mode verification",
        "## False-positive-rate",
        "## Sub-tickets filed",
        "## Conclusion",
    ] {
        assert!(
            doc.contains(section),
            "dogfood-2.md missing required section: {section}"
        );
    }
}

#[test]
fn dogfood_ecosystem_doc_records_all_rust_repos() {
    let doc = read_doc();
    // The per-repo breakdown table must reference every Rust codebase scanned.
    for repo in [
        "wai",
        "dont",
        "pretender",
        "espectacular",
        "testaruda",
        "vampiro",
    ] {
        assert!(
            doc.contains(repo),
            "dogfood-2.md must record the {repo} repo scan"
        );
    }
    // crua and livin (no Rust source) must be explicitly excluded.
    for excluded in ["crua", "livin"] {
        assert!(
            doc.contains(excluded),
            "dogfood-2.md must note the exclusion of {excluded}"
        );
    }
}

#[test]
fn dogfood_ecosystem_doc_records_total_findings_and_fp_rate() {
    let doc = read_doc();
    // The summary table must carry the total-findings count and an FP-rate
    // statement. We check for the tokens, not exact numbers, so the doc can
    // be refreshed across runs without breaking the test.
    assert!(
        doc.contains("Total findings"),
        "dogfood-2.md must record the total findings count"
    );
    assert!(
        doc.contains("False-positive rate"),
        "dogfood-2.md must record the false-positive rate"
    );
}

#[test]
fn dogfood_ecosystem_doc_records_modes() {
    let doc = read_doc();
    // All three scan modes must be exercised and documented.
    for mode in ["guidance", "tiered", "gate"] {
        assert!(
            doc.contains(mode),
            "dogfood-2.md must document the {mode} scan mode"
        );
    }
}

#[test]
fn dogfood_ecosystem_subticket_filed() {
    let doc = read_doc();
    // The frontend facade-leak bug found during dogfood must be filed and
    // referenced in the doc.
    assert!(
        doc.contains("vampiro-03s"),
        "dogfood-2.md must reference the filed sub-ticket vampiro-03s"
    );
}
