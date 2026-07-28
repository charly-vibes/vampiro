use std::path::PathBuf;

#[test]
fn rust_cli_foundation_3_finding_construction() {
    // A finding requires: rule, path, exact line range, configured severity,
    // exactly one axis, and optional filtration_distance
    let finding = vampiro_cli::finding::Finding::new(
        "no-unused-variable",
        PathBuf::from("src/main.rs"),
        10..=15,
        vampiro_cli::finding::Severity::Warning,
        vampiro_cli::finding::Axis::Style,
    );

    assert_eq!(finding.rule, "no-unused-variable");
    assert_eq!(finding.path, PathBuf::from("src/main.rs"));
    assert_eq!(finding.line_range, 10..=15);
    assert_eq!(finding.severity, vampiro_cli::finding::Severity::Warning);
    assert_eq!(finding.axis, vampiro_cli::finding::Axis::Style);
    // Default filtration_distance = Some(sev(Warning)) = Some(2)
    assert_eq!(finding.filtration_distance, Some(2));
}

#[test]
fn rust_cli_foundation_3_finding_custom_filtration_distance() {
    // Filtration distance can be set independently from severity
    let finding = vampiro_cli::finding::Finding::new(
        "critical-bug",
        PathBuf::from("src/core.rs"),
        42..=48,
        vampiro_cli::finding::Severity::Error,
        vampiro_cli::finding::Axis::Correctness,
    )
    .with_filtration_distance(10);

    assert_eq!(finding.filtration_distance, Some(10));
}

#[test]
fn rust_cli_foundation_3_finding_no_filtration_distance() {
    // Filtration distance can be explicitly set to None
    let finding = vampiro_cli::finding::Finding::new(
        "info-message",
        PathBuf::from("src/lib.rs"),
        1..=1,
        vampiro_cli::finding::Severity::Note,
        vampiro_cli::finding::Axis::Performance,
    )
    .with_filtration_distance(None);

    assert_eq!(finding.filtration_distance, None);
}

#[test]
fn rust_cli_foundation_3_finding_serialization() {
    // Finding must be serializable to JSON at the boundary
    let finding = vampiro_cli::finding::Finding::new(
        "no-unused-variable",
        PathBuf::from("src/main.rs"),
        10..=15,
        vampiro_cli::finding::Severity::Warning,
        vampiro_cli::finding::Axis::Style,
    );

    let json = serde_json::to_string_pretty(&finding).unwrap();
    let expected_keys = [
        r#""rule""#,
        r#""path""#,
        r#""line-range-start""#,
        r#""line-range-end""#,
        r#""severity""#,
        r#""axis""#,
        r#""filtration-distance""#,
    ];
    for key in &expected_keys {
        assert!(json.contains(key), "JSON missing key {key}: {json}");
    }
}

#[test]
fn rust_cli_foundation_3_finding_deserialization() {
    // Finding must be deserializable from JSON
    let json = r#"{
        "rule": "no-unused-variable",
        "path": "src/main.rs",
        "line-range-start": 10,
        "line-range-end": 15,
        "severity": "warning",
        "axis": "style",
        "filtration-distance": 2
    }"#;

    let finding: vampiro_cli::finding::Finding =
        serde_json::from_str(json).expect("should deserialize");

    assert_eq!(finding.rule, "no-unused-variable");
    assert_eq!(finding.path, PathBuf::from("src/main.rs"));
    assert_eq!(finding.line_range, 10..=15);
    assert_eq!(finding.severity, vampiro_cli::finding::Severity::Warning);
    assert_eq!(finding.axis, vampiro_cli::finding::Axis::Style);
    assert_eq!(finding.filtration_distance, Some(2));
}

#[test]
fn rust_cli_foundation_3_sev_function() {
    // sev(e) maps severity to numeric value
    use vampiro_cli::finding::Severity;
    assert_eq!(Severity::Error.sev(), 3);
    assert_eq!(Severity::Warning.sev(), 2);
    assert_eq!(Severity::Note.sev(), 1);
}

#[test]
fn rust_cli_foundation_3_axis_display() {
    // Axis should have a human-readable display representation
    use vampiro_cli::finding::Axis;
    assert_eq!(Axis::Correctness.to_string(), "correctness");
    assert_eq!(Axis::Security.to_string(), "security");
    assert_eq!(Axis::Performance.to_string(), "performance");
    assert_eq!(Axis::Style.to_string(), "style");
    assert_eq!(Axis::Safety.to_string(), "safety");
    assert_eq!(Axis::Reliability.to_string(), "reliability");
}
