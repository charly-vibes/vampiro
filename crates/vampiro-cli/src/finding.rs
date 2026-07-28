use std::ops::RangeInclusive;
use std::path::PathBuf;

/// A single finding produced by a Vampiro analysis rule.
///
/// Findings are the atomic unit of analysis output. Each finding
/// identifies a specific location in source code, categorizes it
/// along exactly one axis, and carries a severity and optional
/// filtration distance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Finding {
    /// The rule identifier that produced this finding.
    pub rule: String,
    /// The file path where the finding occurs.
    pub path: PathBuf,
    /// The exact line range of the finding.
    #[serde(flatten)]
    pub line_range: LineRange,
    /// The configured severity level.
    pub severity: Severity,
    /// Exactly one analysis axis.
    pub axis: Axis,
    /// Optional filtration distance, computed as `sev(severity)` by default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filtration_distance: Option<u32>,
}

impl Finding {
    /// Create a new finding with the given parameters.
    ///
    /// `filtration_distance` defaults to `Some(sev(severity))`.
    pub fn new(
        rule: impl Into<String>,
        path: PathBuf,
        line_range: RangeInclusive<usize>,
        severity: Severity,
        axis: Axis,
    ) -> Self {
        let fd = Some(severity.sev());
        Finding {
            rule: rule.into(),
            path,
            line_range: line_range.into(),
            severity,
            axis,
            filtration_distance: fd,
        }
    }

    /// Set a custom filtration distance, overriding the default.
    ///
    /// Pass `Some(n)` to set a specific value, or `None` to omit it entirely.
    pub fn with_filtration_distance(mut self, fd: impl Into<Option<u32>>) -> Self {
        self.filtration_distance = fd.into();
        self
    }
}

/// A line range in source code, serialized as `line-range-start` / `line-range-end`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LineRange {
    /// The first line of the range (1-indexed).
    #[serde(rename = "line-range-start")]
    pub start: usize,
    /// The last line of the range (1-indexed, inclusive).
    #[serde(rename = "line-range-end")]
    pub end: usize,
}

impl From<RangeInclusive<usize>> for LineRange {
    fn from(range: RangeInclusive<usize>) -> Self {
        LineRange {
            start: *range.start(),
            end: *range.end(),
        }
    }
}

impl PartialEq<RangeInclusive<usize>> for LineRange {
    fn eq(&self, other: &RangeInclusive<usize>) -> bool {
        self.start == *other.start() && self.end == *other.end()
    }
}

/// The severity level of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Severity {
    /// A definite error or violation.
    Error,
    /// A potential issue that should be reviewed.
    Warning,
    /// An informational message.
    Note,
}

impl Severity {
    /// Map severity to a numeric value for filtration distance computation.
    ///
    /// - `Error` → 3
    /// - `Warning` → 2
    /// - `Note` → 1
    pub fn sev(self) -> u32 {
        match self {
            Severity::Error => 3,
            Severity::Warning => 2,
            Severity::Note => 1,
        }
    }
}

/// The analysis axis of a finding.
///
/// Each finding is categorized along exactly one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Axis {
    /// Code correctness and logic errors.
    Correctness,
    /// Security vulnerabilities and threats.
    Security,
    /// Performance and efficiency concerns.
    Performance,
    /// Code style and formatting.
    Style,
    /// Safety properties and invariants.
    Safety,
    /// Reliability and robustness.
    Reliability,
}

impl std::fmt::Display for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Axis::Correctness => "correctness",
            Axis::Security => "security",
            Axis::Performance => "performance",
            Axis::Style => "style",
            Axis::Safety => "safety",
            Axis::Reliability => "reliability",
        };
        f.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn finding_construction() {
        let f = Finding::new(
            "test-rule",
            PathBuf::from("src/lib.rs"),
            1..=10,
            Severity::Error,
            Axis::Correctness,
        );
        assert_eq!(f.rule, "test-rule");
        assert_eq!(f.path, PathBuf::from("src/lib.rs"));
        assert_eq!(f.line_range, 1..=10);
        assert_eq!(f.severity, Severity::Error);
        assert_eq!(f.axis, Axis::Correctness);
        assert_eq!(f.filtration_distance, Some(3));
    }

    #[test]
    fn finding_custom_fd() {
        let f = Finding::new(
            "r",
            PathBuf::from("a.rs"),
            1..=1,
            Severity::Note,
            Axis::Style,
        )
        .with_filtration_distance(99);
        assert_eq!(f.filtration_distance, Some(99));
    }

    #[test]
    fn finding_no_fd() {
        let f = Finding::new(
            "r",
            PathBuf::from("a.rs"),
            1..=1,
            Severity::Note,
            Axis::Style,
        )
        .with_filtration_distance(None);
        assert_eq!(f.filtration_distance, None);
    }

    #[test]
    fn severity_sev_values() {
        assert_eq!(Severity::Error.sev(), 3);
        assert_eq!(Severity::Warning.sev(), 2);
        assert_eq!(Severity::Note.sev(), 1);
    }

    #[test]
    fn axis_display() {
        assert_eq!(Axis::Correctness.to_string(), "correctness");
        assert_eq!(Axis::Security.to_string(), "security");
        assert_eq!(Axis::Performance.to_string(), "performance");
        assert_eq!(Axis::Style.to_string(), "style");
        assert_eq!(Axis::Safety.to_string(), "safety");
        assert_eq!(Axis::Reliability.to_string(), "reliability");
    }

    #[test]
    fn finding_serialization() {
        let f = Finding::new(
            "no-unused-variable",
            PathBuf::from("src/main.rs"),
            10..=15,
            Severity::Warning,
            Axis::Style,
        );
        let json = serde_json::to_string_pretty(&f).unwrap();
        assert!(json.contains(r#""rule""#));
        assert!(json.contains(r#""path""#));
        assert!(json.contains(r#""line-range-start""#));
        assert!(json.contains(r#""line-range-end""#));
        assert!(json.contains(r#""severity""#));
        assert!(json.contains(r#""axis""#));
        assert!(json.contains(r#""filtration-distance""#));
    }

    #[test]
    fn finding_deserialization() {
        let json = r#"{
            "rule": "no-unused-variable",
            "path": "src/main.rs",
            "line-range-start": 10,
            "line-range-end": 15,
            "severity": "warning",
            "axis": "style",
            "filtration-distance": 2
        }"#;
        let f: Finding = serde_json::from_str(json).unwrap();
        assert_eq!(f.rule, "no-unused-variable");
        assert_eq!(f.path, PathBuf::from("src/main.rs"));
        assert_eq!(f.line_range, 10..=15);
        assert_eq!(f.severity, Severity::Warning);
        assert_eq!(f.axis, Axis::Style);
        assert_eq!(f.filtration_distance, Some(2));
    }
}
