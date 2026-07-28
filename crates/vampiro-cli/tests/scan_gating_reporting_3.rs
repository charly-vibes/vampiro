//! Integration tests for gating policy and CI generation
//! (add-scan-gating-reporting, section 3).
//!
//! Tests cover:
//! - guidance/tiered/gate mode policy evaluation with real Finding types
//! - Below-threshold and equal-threshold gate behavior
//! - Valid and invalid filtration distance mappings
//! - CI golden templates with explicit PR head/base and failed-fetch behavior

use vampiro_cli::exit_code::ExitCode;
use vampiro_cli::finding::Severity;
use vampiro_cli::output::FlatFinding;
use vampiro_cli::policy::{
    generate_github_actions_workflow, validate_filtration_map, FiltrationMapRule, ScanMode,
    ScanPolicy,
};

/// Helper: create a flat finding from severity string and optional filtration_distance.
fn finding(severity: &str, fd: Option<u32>) -> FlatFinding {
    FlatFinding {
        rule: "REQ-TEST".into(),
        stable_id: "test:1:abc".into(),
        path: "src/test.rs".into(),
        line_range_start: 1,
        line_range_end: 5,
        severity: severity.into(),
        axis: "composition".into(),
        classification: "mismatch".into(),
        filtration_distance: fd,
        evidence: serde_json::json!({}),
    }
}

// ---------------------------------------------------------------------------
// Policy integration tests
// ---------------------------------------------------------------------------

#[test]
fn test_3_1_guidance_passes_with_findings() {
    let policy = ScanPolicy {
        mode: ScanMode::Guidance,
        ..Default::default()
    };
    let findings = vec![finding("high", None)];
    assert_eq!(policy.evaluate(&findings), ExitCode::Success);
}

#[test]
fn test_3_1_tiered_passes_with_findings() {
    let policy = ScanPolicy {
        mode: ScanMode::Tiered,
        ..Default::default()
    };
    let findings = vec![finding("high", None)];
    assert_eq!(policy.evaluate(&findings), ExitCode::Success);
}

#[test]
fn test_3_1_gate_below_threshold_passes() {
    let policy = ScanPolicy {
        mode: ScanMode::Gate,
        severity_threshold: Severity::High,
        ..Default::default()
    };
    let findings = vec![finding("medium", None)];
    assert_eq!(policy.evaluate(&findings), ExitCode::Success);
}

#[test]
fn test_3_1_gate_equal_threshold_fails() {
    let policy = ScanPolicy {
        mode: ScanMode::Gate,
        severity_threshold: Severity::Medium,
        ..Default::default()
    };
    let findings = vec![finding("medium", None)];
    assert_eq!(policy.evaluate(&findings), ExitCode::PolicyFailure);
}

#[test]
fn test_3_1_gate_above_threshold_fails() {
    let policy = ScanPolicy {
        mode: ScanMode::Gate,
        severity_threshold: Severity::Low,
        ..Default::default()
    };
    let findings = vec![finding("high", None)];
    assert_eq!(policy.evaluate(&findings), ExitCode::PolicyFailure);
}

#[test]
fn test_3_1_valid_filtration_map_passes() {
    let rules = vec![
        FiltrationMapRule {
            condition: ">= 2".into(),
            severity: Severity::Low,
        },
        FiltrationMapRule {
            condition: "< 2".into(),
            severity: Severity::High,
        },
    ];
    assert!(validate_filtration_map(&rules).is_ok());
}

#[test]
fn test_3_1_invalid_filtration_map_duplicate_conditions() {
    let rules = vec![
        FiltrationMapRule {
            condition: ">= 2".into(),
            severity: Severity::Low,
        },
        FiltrationMapRule {
            condition: ">= 2".into(),
            severity: Severity::High,
        },
    ];
    assert!(validate_filtration_map(&rules).is_err());
}

#[test]
fn test_3_1_filtration_maps_severity_down() {
    let policy = ScanPolicy {
        mode: ScanMode::Gate,
        severity_threshold: Severity::High,
        filtration_map: Some(vec![FiltrationMapRule {
            condition: ">= 2".into(),
            severity: Severity::Low,
        }]),
        ..Default::default()
    };
    let findings = vec![finding("high", Some(3))];
    // high severity finding with fd=3 gets mapped to low; high threshold passes
    assert_eq!(policy.evaluate(&findings), ExitCode::Success);
}

#[test]
fn test_3_1_filtration_maps_severity_up() {
    let policy = ScanPolicy {
        mode: ScanMode::Gate,
        severity_threshold: Severity::Low,
        filtration_map: Some(vec![FiltrationMapRule {
            condition: ">= 0".into(),
            severity: Severity::High,
        }]),
        ..Default::default()
    };
    let findings = vec![finding("low", Some(0))];
    // low severity gets mapped to high via >= 0; threshold is low → fails
    assert_eq!(policy.evaluate(&findings), ExitCode::PolicyFailure);
}

// ---------------------------------------------------------------------------
// CI generation integration tests
// ---------------------------------------------------------------------------

#[test]
fn test_3_1_ci_golden_github_actions_includes_head_base() {
    let policy = ScanPolicy::default();
    let workflow = generate_github_actions_workflow(&policy).unwrap();
    assert!(
        workflow.contains("pull_request.head.sha"),
        "should include head SHA variable"
    );
    assert!(
        workflow.contains("pull_request.base.sha"),
        "should include base SHA variable"
    );
    assert!(
        workflow.contains("actions/checkout@v4"),
        "should use checkout v4"
    );
    assert!(
        workflow.contains("fetch-depth: 0"),
        "should fetch full depth"
    );
}

#[test]
fn test_3_1_ci_golden_includes_install_step() {
    let policy = ScanPolicy {
        severity_threshold: Severity::High,
        ..Default::default()
    };
    let workflow = generate_github_actions_workflow(&policy).unwrap();
    assert!(workflow.contains("cargo install vampiro"));
    assert!(workflow.contains("--severity-threshold high"));
}

#[test]
fn test_3_1_ci_golden_failed_fetch_handled() {
    // Verify the CI template uses the fallback expression for non-PR events
    let policy = ScanPolicy::default();
    let workflow = generate_github_actions_workflow(&policy).unwrap();
    // The expression `github.event.pull_request.head.sha || github.sha` handles
    // both PR and push events — if head is empty (push event), fallback to sha.
    assert!(
        workflow.contains("github.event.pull_request.head.sha || github.sha"),
        "should handle non-PR events with fallback"
    );
}

#[test]
fn test_3_1_ci_golden_valid_yaml_syntax() {
    let policy = ScanPolicy::default();
    let workflow = generate_github_actions_workflow(&policy).unwrap();
    // Basic check: starts with --- and has a jobs key
    assert!(
        workflow.starts_with("---\n"),
        "workflow should start with YAML frontmatter"
    );
    assert!(
        workflow.contains("jobs:"),
        "workflow should have jobs section"
    );
    assert!(
        workflow.contains("  scan:"),
        "workflow should have scan job"
    );
}

#[test]
fn test_3_1_ci_golden_respects_severity_threshold() {
    let policy = ScanPolicy {
        severity_threshold: Severity::Low,
        ..Default::default()
    };
    let workflow = generate_github_actions_workflow(&policy).unwrap();
    assert!(workflow.contains("--severity-threshold low"));

    let policy_high = ScanPolicy {
        severity_threshold: Severity::High,
        ..Default::default()
    };
    let workflow_high = generate_github_actions_workflow(&policy_high).unwrap();
    assert!(workflow_high.contains("--severity-threshold high"));
}

#[test]
fn test_3_1_empty_findings_in_gate_passes() {
    let policy = ScanPolicy {
        mode: ScanMode::Gate,
        severity_threshold: Severity::Low,
        ..Default::default()
    };
    let findings = vec![];
    assert_eq!(policy.evaluate(&findings), ExitCode::Success);
}

#[test]
fn test_3_1_multiple_findings_gate_fails_on_highest() {
    let policy = ScanPolicy {
        mode: ScanMode::Gate,
        severity_threshold: Severity::Medium,
        ..Default::default()
    };
    let findings = vec![finding("low", None), finding("high", None)];
    assert_eq!(policy.evaluate(&findings), ExitCode::PolicyFailure);
}

#[test]
fn test_3_1_multiple_findings_gate_passes_below() {
    let policy = ScanPolicy {
        mode: ScanMode::Gate,
        severity_threshold: Severity::High,
        ..Default::default()
    };
    let findings = vec![finding("low", None), finding("medium", None)];
    assert_eq!(policy.evaluate(&findings), ExitCode::Success);
}

/// Test that CI golden can be written and validated as rough YAML by
/// a simple parser (just checks structure).
#[test]
fn test_3_1_ci_golden_structural_validity() {
    let policy = ScanPolicy::default();
    let workflow = generate_github_actions_workflow(&policy).unwrap();

    // Structural checks for GitHub Actions format
    assert!(
        workflow.contains("name: Vampiro Scan"),
        "should have workflow name"
    );
    assert!(workflow.contains("on:"), "should have triggers section");
    assert!(workflow.contains("pull_request:"), "should have PR trigger");
    assert!(workflow.contains("push:"), "should have push trigger");
    assert!(workflow.contains("steps:"), "should have steps");
    assert!(
        workflow.contains("Checkout"),
        "should have checkout step name"
    );
}
