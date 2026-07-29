// Finding contract tests — now using the EARS-conformant types from
// vampiro-seam-analysis (REQ-4, v1.3.0).
//
// The closed axis set is {composition, modularity, optionality, robustness}
// and the closed severity vocabulary is {low, medium, high}.

use std::path::PathBuf;
use vampiro_cir::{ScalarKind, Shape};
use vampiro_cli::finding::{Axis, Evidence, Finding, Severity};

#[test]
fn finding_composition_mismatch_construction() {
    // A composition finding (REQ-7) carries rule, path, line range, severity,
    // exactly one axis, and side-by-side shape evidence.
    let finding = Finding::composition_mismatch(
        PathBuf::from("src/main.rs"),
        10..=15,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]),
        vec![Shape::Opaque],
    );

    assert_eq!(finding.rule, "REQ-7");
    assert_eq!(finding.path, PathBuf::from("src/main.rs"));
    assert_eq!(finding.line_range.start, 10);
    assert_eq!(finding.line_range.end, 15);
    assert_eq!(finding.severity, Severity::Medium);
    assert_eq!(finding.axis, Axis::Composition);
    assert_eq!(finding.classification, "composition-break");
}

#[test]
fn finding_serialization() {
    // Finding must be serializable to JSON at the boundary
    let finding = Finding::composition_mismatch(
        PathBuf::from("src/main.rs"),
        10..=15,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]),
        vec![Shape::Opaque],
    );

    let json = serde_json::to_string_pretty(&finding).unwrap();
    let expected_keys = [
        r#""rule""#,
        r#""path""#,
        r#""line-range-start""#,
        r#""line-range-end""#,
        r#""severity""#,
        r#""axis""#,
        r#""classification""#,
        r#""evidence""#,
    ];
    for key in &expected_keys {
        assert!(json.contains(key), "JSON missing key {key}: {json}");
    }
}

#[test]
fn finding_deserialization() {
    // Finding must be deserializable from JSON
    let finding = Finding::composition_mismatch(
        PathBuf::from("src/main.rs"),
        10..=15,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]),
        vec![Shape::Opaque],
    );

    let json = serde_json::to_string(&finding).unwrap();
    let deserialized: Finding = serde_json::from_str(&json).expect("should deserialize");
    assert_eq!(finding, deserialized);
}

#[test]
fn axis_display_uses_closed_set() {
    assert_eq!(Axis::Composition.to_string(), "composition");
    assert_eq!(Axis::Modularity.to_string(), "modularity");
    assert_eq!(Axis::Optionality.to_string(), "optionality");
    assert_eq!(Axis::Robustness.to_string(), "robustness");
}

#[test]
fn severity_uses_lowercase_vocabulary() {
    assert_eq!(
        serde_json::to_value(Severity::Low).unwrap(),
        serde_json::json!("low")
    );
    assert_eq!(
        serde_json::to_value(Severity::Medium).unwrap(),
        serde_json::json!("medium")
    );
    assert_eq!(
        serde_json::to_value(Severity::High).unwrap(),
        serde_json::json!("high")
    );
}

#[test]
fn evidence_carries_side_by_side_shapes() {
    let finding = Finding::composition_mismatch(
        PathBuf::from("src/lib.rs"),
        1..=1,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]),
        vec![Shape::Opaque],
    );
    #[allow(irrefutable_let_patterns)]
    let Evidence::CompositionMismatch {
        caller_expected,
        callee_produced,
        unhandled,
    } = &finding.evidence
    else {
        panic!("expected composition mismatch evidence");
    };
    assert_eq!(caller_expected, &Shape::Scalar(ScalarKind::Unit));
    assert_ne!(caller_expected, callee_produced);
    assert_eq!(unhandled, &vec![Shape::Opaque]);
}
