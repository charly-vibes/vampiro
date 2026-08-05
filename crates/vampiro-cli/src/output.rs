//! Normalized scan result and three renderers (REQ-15, REQ-19, REQ-24, REQ-C2).
//!
//! Single `ScanResult` type from which all output formats (human, JSON, SARIF)
//! are derived. Each finding gets a stable deduplication ID from rule +
//! location + shape hash (REQ-24). Unanalyzed files are explicitly listed
//! (REQ-15).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use vampiro_seam_analysis::{Diagnostic, Evidence, Finding};

pub use scan_result::*;

mod scan_result {
    use super::*;

    /// The normalized result of a scan. One source of truth for all renderers.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ScanResult {
        /// A human-readable name for this scan run.
        pub name: String,
        /// All findings from the scan.
        pub findings: Vec<FlatFinding>,
        /// All diagnostics from the scan.
        pub diagnostics: Vec<FlatDiagnostic>,
        /// File paths that could not be analyzed (no frontend available).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub unanalyzed: Vec<String>,
        /// Metadata about the scan (scope, commits, etc.).
        pub metadata: ScanResultMetadata,
    }

    /// Flat serializable representation of a finding for all output formats.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FlatFinding {
        /// The rule identifier (e.g. "REQ-7").
        pub rule: String,
        /// The stable deduplication ID (REQ-24).
        #[serde(rename = "stable-id")]
        pub stable_id: String,
        /// File path relative to workspace root.
        pub path: String,
        /// Line range start (1-indexed).
        #[serde(rename = "line-range-start")]
        pub line_range_start: usize,
        /// Line range end (1-indexed, inclusive).
        #[serde(rename = "line-range-end")]
        pub line_range_end: usize,
        /// Severity level: low, medium, high.
        pub severity: String,
        /// Analysis axis.
        pub axis: String,
        /// Classification.
        pub classification: String,
        /// Filtration distance, if computed.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub filtration_distance: Option<u32>,
        /// Rule-specific evidence as raw JSON value.
        pub evidence: serde_json::Value,
    }

    /// Flat serializable representation of a diagnostic.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct FlatDiagnostic {
        pub diagnostic: String,
        pub path: String,
        #[serde(rename = "line-range-start")]
        pub line_range_start: usize,
        #[serde(rename = "line-range-end")]
        pub line_range_end: usize,
        pub detail: String,
    }

    /// Metadata about a scan.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ScanResultMetadata {
        /// Scope of the scan.
        pub scope: ScopeKind,
        /// Base commit (for diff scans).
        #[serde(skip_serializing_if = "Option::is_none")]
        pub base_commit: Option<String>,
        /// Target commit (for diff scans).
        #[serde(skip_serializing_if = "Option::is_none")]
        pub target_commit: Option<String>,
        /// Number of files scanned.
        pub scanned_files: usize,
    }

    /// Whether the scan was diff-scoped or full.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum ScopeKind {
        Diff,
        Full,
    }

    impl std::fmt::Display for ScopeKind {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ScopeKind::Diff => f.write_str("diff"),
                ScopeKind::Full => f.write_str("full"),
            }
        }
    }

    impl ScanResult {
        /// Create a new normalized scan result.
        pub fn new(
            name: String,
            findings: Vec<Finding>,
            diagnostics: Vec<Diagnostic>,
            unanalyzed: Vec<PathBuf>,
            metadata: ScanResultMetadata,
        ) -> Self {
            let flat_findings: Vec<FlatFinding> = findings
                .into_iter()
                .map(|f| {
                    let stable_id = Self::stable_id_for_finding(&f);
                    let evidence =
                        serde_json::to_value(&f.evidence).unwrap_or(serde_json::Value::Null);
                    FlatFinding {
                        rule: f.rule,
                        stable_id,
                        path: f.path.to_string_lossy().to_string(),
                        line_range_start: f.line_range.start,
                        line_range_end: f.line_range.end,
                        severity: f.severity.to_string(),
                        axis: f.axis.to_string(),
                        classification: f.classification,
                        filtration_distance: f.filtration_distance,
                        evidence,
                    }
                })
                .collect();

            let flat_diagnostics: Vec<FlatDiagnostic> = diagnostics
                .into_iter()
                .map(|d| FlatDiagnostic {
                    diagnostic: d.diagnostic,
                    path: d.path.to_string_lossy().to_string(),
                    line_range_start: d.line_range.start,
                    line_range_end: d.line_range.end,
                    detail: d.detail,
                })
                .collect();

            let unanalyzed: Vec<String> = unanalyzed
                .into_iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            ScanResult {
                name,
                findings: flat_findings,
                diagnostics: flat_diagnostics,
                unanalyzed,
                metadata,
            }
        }

        /// Compute a stable deduplication ID for a finding (REQ-24).
        ///
        /// Derived from rule + path + line range + shape hashes in evidence.
        /// This is deterministic across full and diff scans.
        pub fn stable_id_for_finding(finding: &Finding) -> String {
            let evidence_hash = match &finding.evidence {
                Evidence::CompositionMismatch {
                    caller_expected,
                    callee_produced,
                    ..
                } => {
                    let ce = serde_json::to_string(caller_expected).unwrap_or_default();
                    let cp = serde_json::to_string(callee_produced).unwrap_or_default();
                    hash_string(&format!("{}{}", ce, cp))
                }
                _ => {
                    let json = serde_json::to_string(&finding.evidence).unwrap_or_default();
                    hash_string(&json)
                }
            };

            let raw = format!(
                "{}:{}:{}:{}:{}",
                finding.rule,
                finding.path.to_string_lossy(),
                finding.line_range.start,
                finding.line_range.end,
                evidence_hash,
            );
            let full_hash = hash_string(&raw);
            format!(
                "{}:{}:{}",
                finding.rule,
                finding.line_range.start,
                &full_hash[..8]
            )
        }
    }
}

/// Compute a hex SHA-256 hash of a string, truncated to 16 hex chars (64 bits).
fn hash_string(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

// ---------------------------------------------------------------------------
// Renderers
// ---------------------------------------------------------------------------

/// Render the scan result as human-readable text (REQ-19).
pub fn render_human(result: &ScanResult) -> String {
    use std::fmt::Write;
    let mut out = String::new();

    writeln!(out, "vampiro check — {}", result.name).ok();
    writeln!(
        out,
        "Scope: {} | Files scanned: {}",
        result.metadata.scope, result.metadata.scanned_files
    )
    .ok();

    if let Some(base) = &result.metadata.base_commit {
        writeln!(out, "Base:   {}", base).ok();
    }
    if let Some(target) = &result.metadata.target_commit {
        writeln!(out, "Target: {}", target).ok();
    }

    if result.findings.is_empty() && result.diagnostics.is_empty() {
        writeln!(out, "\nNo findings, no diagnostics.").ok();
        if !result.unanalyzed.is_empty() {
            writeln!(out, "\nUnanalyzed files ({}):", result.unanalyzed.len()).ok();
            for u in &result.unanalyzed {
                writeln!(out, "  {}", u).ok();
            }
        }
        return out;
    }

    for f in &result.findings {
        writeln!(
            out,
            "\n{}:{}-{}  {} [{}]  {}  ({})",
            f.path,
            f.line_range_start,
            f.line_range_end,
            f.rule,
            f.severity,
            f.classification,
            f.axis,
        )
        .ok();
        writeln!(out, "  stable-id: {}", f.stable_id).ok();
        if let Some(fd) = f.filtration_distance {
            writeln!(out, "  filtration-distance: {}", fd).ok();
        }
    }

    for d in &result.diagnostics {
        writeln!(
            out,
            "\n{}:{}-{}  {}  {}",
            d.path, d.line_range_start, d.line_range_end, d.diagnostic, d.detail,
        )
        .ok();
    }

    if !result.unanalyzed.is_empty() {
        writeln!(out, "\nUnanalyzed files ({}):", result.unanalyzed.len()).ok();
        for u in &result.unanalyzed {
            writeln!(out, "  {}", u).ok();
        }
    }

    writeln!(
        out,
        "\n{} finding(s), {} diagnostic(s), {} unanalyzed file(s) in {} file(s)",
        result.findings.len(),
        result.diagnostics.len(),
        result.unanalyzed.len(),
        result.metadata.scanned_files,
    )
    .ok();

    out
}

/// Render the scan result as JSON (genesis envelope) (REQ-19).
pub fn render_json(result: &ScanResult) -> Result<String, Box<dyn std::error::Error>> {
    let all_findings: Vec<serde_json::Value> = result
        .findings
        .iter()
        .map(|f| serde_json::to_value(f).unwrap_or(serde_json::Value::Null))
        .collect();

    let all_warnings: Vec<genesis::envelope::Warning> = result
        .diagnostics
        .iter()
        .map(|d| {
            let entity_id = format!("{}:{}", d.path, d.line_range_start);
            genesis::envelope::Warning {
                rule_name: d.diagnostic.clone(),
                entity_id: Some(entity_id),
                message: d.detail.clone(),
                suggested_remediation: None,
            }
        })
        .collect();

    let unanalyzed: Vec<String> = result.unanalyzed.clone();
    let hints: Vec<genesis::envelope::HintEntry> = unanalyzed
        .into_iter()
        .map(|u| genesis::envelope::HintEntry {
            command: "unanalyzed".into(),
            description: u,
        })
        .collect();

    let env = genesis::envelope::Envelope::success(
        env!("CARGO_PKG_VERSION"),
        genesis::envelope::EnvelopeKind::Check,
        all_findings,
        all_warnings,
        hints,
    );
    Ok(serde_json::to_string_pretty(&env)?)
}

/// Render the scan result as SARIF 2.1.0 (REQ-19).
pub fn render_sarif(result: &ScanResult) -> Result<String, Box<dyn std::error::Error>> {
    let sarif = serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "vampiro",
                        "version": env!("CARGO_PKG_VERSION"),
                        "informationUri": "https://github.com/charly-vibes/vampiro"
                    }
                },
                "results": result.findings.iter().map(|f| {
                    serde_json::json!({
                        "ruleId": f.rule,
                        "message": {
                            "text": format!("{}: {}", f.classification, f.axis)
                        },
                        "locations": [
                            {
                                "physicalLocation": {
                                    "artifactLocation": {
                                        "uri": f.path
                                    },
                                    "region": {
                                        "startLine": f.line_range_start,
                                        "endLine": f.line_range_end
                                    }
                                }
                            }
                        ],
                        "properties": {
                            "severity": f.severity,
                            "stable-id": f.stable_id,
                            "axis": f.axis,
                            "classification": f.classification,
                            "evidence": f.evidence
                        }
                    })
                }).collect::<Vec<serde_json::Value>>(),
                "taxonomies": [
                    {
                        "name": "vampiro-axes",
                        "description": {
                            "text": "Vampiro analysis axes: composition, modularity, optionality, robustness"
                        }
                    }
                ]
            }
        ]
    });
    Ok(serde_json::to_string_pretty(&sarif)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_kind_serializes_correctly() {
        assert_eq!(
            serde_json::to_value(ScopeKind::Diff).unwrap(),
            serde_json::json!("diff")
        );
        assert_eq!(
            serde_json::to_value(ScopeKind::Full).unwrap(),
            serde_json::json!("full")
        );
    }

    #[test]
    fn hash_string_is_deterministic() {
        let h1 = hash_string("hello world");
        let h2 = hash_string("hello world");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
    }

    #[test]
    fn hash_string_differs_for_different_inputs() {
        let h1 = hash_string("abc");
        let h2 = hash_string("xyz");
        assert_ne!(h1, h2);
    }
}
