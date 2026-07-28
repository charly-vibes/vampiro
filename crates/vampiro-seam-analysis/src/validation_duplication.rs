//! Validation-duplication analysis: detects when the same validation check
//! appears at multiple locations outside the recognized smart constructor.
//!
//! A validation duplication occurs when:
//! 1. A frontend extracts a `ValidationObservation` with a stable identity
//! 2. A node outside the recognized smart constructor repeats validation
//!    tied to the same stable identity
//!
//! Validation equivalence is established ONLY through a shared stable
//! validation identity (from declaration or conformance-tested idiom).
//! Mere syntactic similarity without identity evidence emits no finding.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::finding::{Axis, Evidence, Finding, Severity};

/// Analyzer for validation-duplication findings (REQ-B4).
#[derive(Debug, Default)]
pub struct ValidationDuplicationAnalyzer;

impl ValidationDuplicationAnalyzer {
    /// Create a new validation-duplication analyzer.
    pub fn new() -> Self {
        ValidationDuplicationAnalyzer
    }

    /// Analyze the graph for validation-duplication findings.
    ///
    /// For each validation observation, checks if the check location is
    /// outside the recognized smart constructor's source span. If so,
    /// emits exactly one `LOW` modularity `validation-duplication` finding
    /// per duplicate-check location.
    pub fn analyze(&self, graph: &vampiro_cir::CirGraph) -> Vec<Finding> {
        if graph.validation_observations.is_empty() {
            return Vec::new();
        }

        // Group observations by validation identity
        let mut by_identity: HashMap<&str, Vec<&vampiro_cir::ValidationObservation>> =
            HashMap::new();
        for obs in &graph.validation_observations {
            by_identity
                .entry(obs.identity.as_str())
                .or_default()
                .push(obs);
        }

        let mut findings = Vec::new();

        for observations in by_identity.values() {
            if observations.len() < 2 {
                continue;
            }

            // Find the constructor's own observation (closest to the
            // constructor's definition). For now, we treat the first
            // observation as the "primary" and all others as duplicates.
            let _primary = observations[0];

            for obs in &observations[1..] {
                let path = PathBuf::from(&obs.span.file);
                let line_range = obs.span.start_line..=obs.span.end_line;

                let finding = Finding {
                    rule: "REQ-B4".into(),
                    path,
                    line_range: line_range.into(),
                    severity: Severity::Low,
                    axis: Axis::Modularity,
                    filtration_distance: None,
                    evidence: Evidence::ValidationDuplication {
                        identity: obs.identity.clone(),
                        constructor_id: obs.constructor_id.to_string(),
                        refined_shape: obs.refined_shape.clone(),
                        origin: obs.origin.clone(),
                    },
                    classification: "validation-duplication".into(),
                };
                findings.push(finding);
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vampiro_cir::{CirGraph, SourceSpan, StableId, ValidationObservation};

    fn obs(
        identity: &str,
        constructor: &str,
        shape: &str,
        file: &str,
        line: usize,
        origin: &str,
    ) -> ValidationObservation {
        ValidationObservation {
            identity: identity.into(),
            constructor_id: StableId::new(constructor),
            refined_shape: shape.into(),
            span: SourceSpan {
                file: file.into(),
                start_line: line,
                start_column: 1,
                end_line: line,
                end_column: 20,
            },
            origin: origin.into(),
        }
    }

    #[test]
    fn single_observation_emits_no_finding() {
        let mut graph = CirGraph::new("lib.rs");
        graph.validation_observations.push(obs(
            "validate_user",
            "User::new",
            "User",
            "lib.rs",
            10,
            "idiom",
        ));

        let findings = ValidationDuplicationAnalyzer::new().analyze(&graph);
        assert!(
            findings.is_empty(),
            "single observation should not trigger duplication"
        );
    }

    #[test]
    fn duplicate_validation_emits_finding() {
        let mut graph = CirGraph::new("lib.rs");
        // Primary observation at the constructor
        graph.validation_observations.push(obs(
            "validate_user",
            "User::new",
            "User",
            "lib.rs",
            10,
            "idiom",
        ));
        // Duplicate observation at a different location
        graph.validation_observations.push(obs(
            "validate_user",
            "User::new",
            "User",
            "lib.rs",
            50,
            "idiom",
        ));

        let findings = ValidationDuplicationAnalyzer::new().analyze(&graph);
        assert_eq!(
            findings.len(),
            1,
            "expected one validation-duplication finding"
        );

        let f = &findings[0];
        assert_eq!(f.rule, "REQ-B4");
        assert_eq!(f.severity, Severity::Low);
        assert_eq!(f.axis, Axis::Modularity);
        assert_eq!(f.classification, "validation-duplication");

        match &f.evidence {
            Evidence::ValidationDuplication {
                identity,
                constructor_id,
                refined_shape,
                origin,
            } => {
                assert_eq!(identity, "validate_user");
                assert_eq!(constructor_id, &StableId::new("User::new").to_string());
                assert_eq!(refined_shape, "User");
                assert_eq!(origin, "idiom");
            }
            other => panic!("expected ValidationDuplication evidence, got {other:?}"),
        }
    }

    #[test]
    fn three_duplicates_emit_two_findings() {
        let mut graph = CirGraph::new("lib.rs");
        // Primary + 2 duplicates
        graph.validation_observations.push(obs(
            "vid",
            "Ctor",
            "Shape",
            "lib.rs",
            10,
            "declaration",
        ));
        graph.validation_observations.push(obs(
            "vid",
            "Ctor",
            "Shape",
            "lib.rs",
            50,
            "declaration",
        ));
        graph.validation_observations.push(obs(
            "vid",
            "Ctor",
            "Shape",
            "lib.rs",
            80,
            "declaration",
        ));

        let findings = ValidationDuplicationAnalyzer::new().analyze(&graph);
        assert_eq!(
            findings.len(),
            2,
            "expected two findings for 3 observations"
        );
    }

    #[test]
    fn different_identities_do_not_collide() {
        let mut graph = CirGraph::new("lib.rs");
        graph
            .validation_observations
            .push(obs("a", "Ctor1", "A", "lib.rs", 10, "declaration"));
        graph
            .validation_observations
            .push(obs("a", "Ctor1", "A", "lib.rs", 50, "declaration"));
        graph
            .validation_observations
            .push(obs("b", "Ctor2", "B", "lib.rs", 20, "idiom"));
        graph
            .validation_observations
            .push(obs("b", "Ctor2", "B", "lib.rs", 60, "idiom"));

        let findings = ValidationDuplicationAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 2, "expected 2 findings (one per identity)");
    }

    #[test]
    fn empty_observations_emits_no_finding() {
        let graph = CirGraph::new("lib.rs");
        let findings = ValidationDuplicationAnalyzer::new().analyze(&graph);
        assert!(findings.is_empty());
    }

    #[test]
    fn different_identity_no_duplicate() {
        let mut graph = CirGraph::new("lib.rs");
        graph
            .validation_observations
            .push(obs("a", "Ctor1", "A", "lib.rs", 10, "idiom"));
        graph
            .validation_observations
            .push(obs("b", "Ctor2", "B", "lib.rs", 20, "idiom"));

        let findings = ValidationDuplicationAnalyzer::new().analyze(&graph);
        assert!(
            findings.is_empty(),
            "different identities should not be duplicates"
        );
    }
}
