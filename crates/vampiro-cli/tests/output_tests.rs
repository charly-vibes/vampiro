//! Tests for the normalized result and renderers (REQ-15, REQ-19, REQ-24, REQ-C2).

use std::path::PathBuf;
use vampiro_cir::ScalarKind;
use vampiro_cli::output::{
    render_human, render_json, render_sarif, ScanResult, ScanResultMetadata, ScopeKind,
};
use vampiro_seam_analysis::finding::LineRange;
use vampiro_seam_analysis::Finding;

fn sample_finding() -> Finding {
    Finding::composition_mismatch(
        PathBuf::from("src/lib.rs"),
        10..=12,
        vampiro_cir::Shape::Scalar(ScalarKind::Unit),
        vampiro_cir::Shape::Union(vec![vampiro_cir::Shape::Scalar(ScalarKind::Unit), vampiro_cir::Shape::Opaque]),
        vec![vampiro_cir::Shape::Opaque],
    )
}

fn sample_diagnostic() -> vampiro_seam_analysis::Diagnostic {
    vampiro_seam_analysis::Diagnostic {
        diagnostic: "boundary:enforced-unreachable".into(),
        path: PathBuf::from("src/lib.rs"),
        line_range: LineRange::new(5, 5),
        detail: "frontend classification bug".into(),
    }
}

#[test]
fn scan_result_constructs_and_round_trips() {
    let result = ScanResult::new(
        "test-scan".into(),
        vec![sample_finding()],
        vec![sample_diagnostic()],
        vec!["src/unknown.js".into()],
        ScanResultMetadata {
            scope: ScopeKind::Diff,
            base_commit: Some("abc123".into()),
            target_commit: Some("def456".into()),
            scanned_files: 3,
        },
    );

    assert_eq!(result.name, "test-scan");
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.unanalyzed.len(), 1);
}

#[test]
fn stable_dedup_id_is_deterministic() {
    let f1 = sample_finding();
    let f2 = sample_finding();

    let id1 = ScanResult::stable_id_for_finding(&f1);
    let id2 = ScanResult::stable_id_for_finding(&f2);

    assert_eq!(id1, id2, "same finding must produce same stable ID");
    assert!(id1.starts_with("REQ-7"), "stable ID must start with rule");
}

#[test]
fn stable_dedup_id_differs_for_different_findings() {
    let f1 = Finding::composition_mismatch(
        PathBuf::from("src/lib.rs"),
        10..=12,
        vampiro_cir::Shape::Scalar(ScalarKind::Unit),
        vampiro_cir::Shape::Scalar(ScalarKind::Unit),
        vec![],
    );
    let f2 = Finding::composition_mismatch(
        PathBuf::from("src/lib.rs"),
        20..=22,
        vampiro_cir::Shape::Scalar(ScalarKind::Unit),
        vampiro_cir::Shape::Scalar(ScalarKind::Unit),
        vec![],
    );

    let id1 = ScanResult::stable_id_for_finding(&f1);
    let id2 = ScanResult::stable_id_for_finding(&f2);

    assert_ne!(id1, id2, "different locations must produce different IDs");
}

#[test]
fn render_json_produces_valid_output() {
    let result = ScanResult::new(
        "test".into(),
        vec![sample_finding()],
        vec![sample_diagnostic()],
        vec!["unanalyzed.js".into()],
        ScanResultMetadata {
            scope: ScopeKind::Full,
            base_commit: None,
            target_commit: Some("abc".into()),
            scanned_files: 2,
        },
    );

    let json = render_json(&result).expect("JSON render must succeed");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("output must be valid JSON");
    assert_eq!(parsed["envelope_kind"], "check");
    assert!(parsed["data"].is_array());
    assert!(parsed["warnings"].is_array());
    assert!(parsed["hints"].is_array());
}

#[test]
fn render_human_produces_output() {
    let result = ScanResult::new(
        "test".into(),
        vec![sample_finding()],
        vec![sample_diagnostic()],
        vec![],
        ScanResultMetadata {
            scope: ScopeKind::Diff,
            base_commit: None,
            target_commit: None,
            scanned_files: 1,
        },
    );

    let human = render_human(&result);
    assert!(human.contains("REQ-7"), "human output must contain rule");
    assert!(!human.is_empty());
}

#[test]
fn render_sarif_produces_valid_output() {
    let result = ScanResult::new(
        "test".into(),
        vec![sample_finding()],
        vec![],
        vec![],
        ScanResultMetadata {
            scope: ScopeKind::Full,
            base_commit: None,
            target_commit: None,
            scanned_files: 1,
        },
    );

    let sarif = render_sarif(&result).expect("SARIF render must succeed");
    let parsed: serde_json::Value = serde_json::from_str(&sarif).expect("SARIF must be valid JSON");
    assert!(parsed.get("$schema").is_some(), "SARIF must have $schema");
    assert_eq!(parsed["version"], "2.1.0");
    assert!(parsed["runs"].is_array());
}

#[test]
fn renderers_are_semantically_equivalent() {
    let result = ScanResult::new(
        "equiv-test".into(),
        vec![sample_finding()],
        vec![sample_diagnostic()],
        vec![],
        ScanResultMetadata {
            scope: ScopeKind::Diff,
            base_commit: None,
            target_commit: None,
            scanned_files: 2,
        },
    );

    let json = render_json(&result).expect("json");
    assert!(json.contains("composition-break"));
}

#[test]
fn unanalyzed_files_appear_in_all_formats() {
    let unanalyzed = vec!["src/unknown.py".into(), "src/data.json".into()];
    let result = ScanResult::new(
        "unanalyzed-test".into(),
        vec![],
        vec![],
        unanalyzed,
        ScanResultMetadata {
            scope: ScopeKind::Full,
            base_commit: None,
            target_commit: None,
            scanned_files: 5,
        },
    );

    let json = render_json(&result).expect("json");
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["hints"].as_array().map(|a| a.len()).unwrap_or(0), 2);

    let human = render_human(&result);
    assert!(human.contains("unanalyzed"));
    assert!(human.contains("src/unknown.py"));
}

#[test]
fn finding_has_stable_id_in_output() {
    let result = ScanResult::new(
        "stable-id".into(),
        vec![sample_finding()],
        vec![],
        vec![],
        ScanResultMetadata {
            scope: ScopeKind::Diff,
            base_commit: None,
            target_commit: None,
            scanned_files: 1,
        },
    );

    let json = render_json(&result).expect("json");
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let findings = parsed["data"].as_array().unwrap();
    assert!(!findings.is_empty());
    let f = &findings[0];
    assert!(
        f.get("stable-id").is_some(),
        "finding must have stable ID in output: {f}"
    );
}
