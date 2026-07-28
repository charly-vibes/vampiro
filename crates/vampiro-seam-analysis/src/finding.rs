//! The normalized finding contract (REQ-4, EARS v1.3.0).
//!
//! Findings are the atomic unit of analysis output. Each finding identifies a
//! specific source location, categorizes it along exactly one of the four
//! closed axes, carries a severity from the closed `{LOW, MEDIUM, HIGH}`
//! vocabulary, an optional filtration distance, and a rule-specific evidence
//! payload.
//!
//! # Default severities
//!
//! Per the REQ-4 default-severity table, each rule has a documented default
//! severity, overridable by project configuration. The composition rule
//! (`REQ-7`) defaults to `MEDIUM`. Diagnostics (e.g.
//! `boundary:enforced-unreachable`, `identity:hash-collision`) carry no
//! severity and no axis; they are a separate `Diagnostic` type introduced in
//! a later slice.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use vampiro_cir::Shape;

/// The closed set of analysis axes (REQ-4, REQ-4b finding taxonomy).
///
/// These four values are the complete and exact set; slash notation such as
/// `robustness/composition` is prohibited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Axis {
    /// Shape legitimacy / composition break (REQ-7, REQ-T4).
    Composition,
    /// Visibility / reach-through / facade (REQ-8, REQ-V4–V7).
    Modularity,
    /// Algebraic-law satisfaction / substitutability (REQ-10).
    Optionality,
    /// Effect totality, retries, resource linearity, redundancy (REQ-9, REQ-11).
    Robustness,
}

impl std::fmt::Display for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Axis::Composition => f.write_str("composition"),
            Axis::Modularity => f.write_str("modularity"),
            Axis::Optionality => f.write_str("optionality"),
            Axis::Robustness => f.write_str("robustness"),
        }
    }
}

/// The closed severity vocabulary (REQ-4, v1.3.0).
///
/// JSON form is lowercase (`low`/`medium`/`high`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Low => f.write_str("low"),
            Severity::Medium => f.write_str("medium"),
            Severity::High => f.write_str("high"),
        }
    }
}

/// A line range in source code, serialized as `line-range-start` /
/// `line-range-end`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineRange {
    /// The first line of the range (1-indexed).
    #[serde(rename = "line-range-start")]
    pub start: usize,
    /// The last line of the range (1-indexed, inclusive).
    #[serde(rename = "line-range-end")]
    pub end: usize,
}

impl LineRange {
    /// Construct a line range from an inclusive `start..=end`.
    pub fn new(start: usize, end: usize) -> Self {
        LineRange { start, end }
    }
}

impl From<std::ops::RangeInclusive<usize>> for LineRange {
    fn from(range: std::ops::RangeInclusive<usize>) -> Self {
        LineRange {
            start: *range.start(),
            end: *range.end(),
        }
    }
}

/// Rule-specific evidence carried by a finding.
///
/// Only the composition mismatch variant exists today; modularity,
/// optionality, and robustness evidence variants are added by tasks
/// `0vb.4.3`–`0vb.4.5`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Evidence {
    /// REQ-7: a callee's codomain shape does not structurally unify with the
    /// caller's expected domain shape.
    ///
    /// Carries both shapes side by side (the spec requires the finding to
    /// "show the caller-expected shape and the callee-produced shape side by
    /// side") plus the arms the caller left unhandled.
    CompositionMismatch {
        /// The shape the caller's domain expected.
        caller_expected: Shape,
        /// The shape the callee's codomain produced.
        callee_produced: Shape,
        /// Arms of the produced union (if any) the caller left unhandled.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        unhandled: Vec<Shape>,
    },
}

/// One reported issue (REQ-4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// The rule identifier that produced this finding (e.g. `"REQ-7"`).
    pub rule: String,
    /// The file path where the finding occurs.
    pub path: PathBuf,
    /// The exact line range of the finding.
    #[serde(flatten)]
    pub line_range: LineRange,
    /// The configured severity level (REQ-4 default-severity table).
    pub severity: Severity,
    /// Exactly one analysis axis.
    pub axis: Axis,
    /// Optional filtration distance (REQ-C2), computed from a declared
    /// filtration. Absent when no filtration is declared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filtration_distance: Option<u32>,
    /// Rule-specific evidence.
    pub evidence: Evidence,
}

impl Finding {
    /// Build a composition-break finding (REQ-7) with the default severity
    /// (`MEDIUM`) and the supplied side-by-side shapes.
    pub fn composition_mismatch(
        path: PathBuf,
        line_range: impl Into<LineRange>,
        caller_expected: Shape,
        callee_produced: Shape,
        unhandled: Vec<Shape>,
    ) -> Self {
        Finding {
            rule: "REQ-7".into(),
            path,
            line_range: line_range.into(),
            severity: Severity::Medium,
            axis: Axis::Composition,
            filtration_distance: None,
            evidence: Evidence::CompositionMismatch {
                caller_expected: caller_expected.normalize(),
                callee_produced: callee_produced.normalize(),
                unhandled: unhandled.into_iter().map(|s| s.normalize()).collect(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vampiro_cir::Shape;

    #[test]
    fn axis_serializes_to_closed_kebab_set() {
        assert_eq!(
            serde_json::to_value(Axis::Composition).unwrap(),
            serde_json::json!("composition")
        );
        assert_eq!(
            serde_json::to_value(Axis::Modularity).unwrap(),
            serde_json::json!("modularity")
        );
        assert_eq!(
            serde_json::to_value(Axis::Optionality).unwrap(),
            serde_json::json!("optionality")
        );
        assert_eq!(
            serde_json::to_value(Axis::Robustness).unwrap(),
            serde_json::json!("robustness")
        );
    }

    #[test]
    fn severity_serializes_to_lowercase() {
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
    #[allow(irrefutable_let_patterns)]
    fn composition_finding_default_severity_is_medium() {
        let f = Finding::composition_mismatch(
            PathBuf::from("src/lib.rs"),
            10..=12,
            Shape::Scalar,
            Shape::Union(vec![Shape::Scalar, Shape::Opaque]),
            vec![Shape::Opaque],
        );
        assert_eq!(f.rule, "REQ-7");
        assert_eq!(f.severity, Severity::Medium);
        assert_eq!(f.axis, Axis::Composition);
        assert_eq!(f.line_range, LineRange::new(10, 12));
        assert!(f.filtration_distance.is_none());
        // Evidence carries both shapes side by side (REQ-7).
        #[allow(irrefutable_let_patterns)]
        let Evidence::CompositionMismatch {
            caller_expected,
            callee_produced,
            unhandled,
        } = &f.evidence
        else {
            panic!("expected composition mismatch evidence");
        };
        assert_eq!(caller_expected, &Shape::Scalar);
        assert_eq!(
            callee_produced,
            &Shape::Union(vec![Shape::Opaque, Shape::Scalar])
        );
        assert_eq!(unhandled, &vec![Shape::Opaque]);
    }
}
