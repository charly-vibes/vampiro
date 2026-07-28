//! Retry idempotency analysis (REQ-T2, REQ-T5, REQ-T6, REQ-T9).
//!
//! Classifies retry edges by their write shape's idempotency class and
//! produces:
//! - `unsafe-retry` findings (REQ-T5) for `non-idempotent` retry edges
//! - `idempotency-coverage-unknown` diagnostics (REQ-T9) for unknown idioms
//! - Cross-references with law evidence (REQ-T6)

use serde::{Deserialize, Serialize};

use crate::write_idiom_table::{IdempotencyClass, WriteIdiomTable};

/// The current retry-idempotency analysis schema version.
pub const RETRY_IDEMPOTENCY_SCHEMA_VERSION: &str = "0.1.0";

// ---------------------------------------------------------------------------
// RetryFact (simplified input for analysis)
// ---------------------------------------------------------------------------

/// A retry fact extracted from source code — the input to idempotency analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RetryIdempotencyFact {
    /// Source file path.
    pub source_file: String,
    /// Line number of the retry construct.
    pub line: usize,
    /// The function containing this retry.
    pub function: String,
    /// The write operation being retried (method/function name).
    pub write_method: String,
    /// Kind of retry pattern (e.g. "loop", "while", "retry-library").
    pub retry_kind: String,
}

// ---------------------------------------------------------------------------
// RetryIdempotencyFinding
// ---------------------------------------------------------------------------

/// A finding from retry idempotency analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RetryIdempotencyFinding {
    /// Source file path.
    pub source_file: String,
    /// Line number.
    pub line: usize,
    /// The function containing the retry.
    pub function: String,
    /// The write operation being retried.
    pub write_method: String,
    /// The classification that triggered this finding.
    pub classification: String,
    /// Human-readable description.
    pub detail: String,
}

// ---------------------------------------------------------------------------
// RetryIdempotencyDiagnostic
// ---------------------------------------------------------------------------

/// A coverage diagnostic (REQ-T9) — not a finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RetryCoverageDiagnostic {
    /// Source file path.
    pub source_file: String,
    /// Line number.
    pub line: usize,
    /// The write operation with unknown classification.
    pub write_method: String,
    /// The idiom table version consulted.
    pub table_version: String,
}

// ---------------------------------------------------------------------------
// RetryIdempotencyAnalyzer
// ---------------------------------------------------------------------------

/// Analyzes retry facts against a write-shape idiom table for idempotency
/// violations.
#[derive(Debug, Clone)]
pub struct RetryIdempotencyAnalyzer {
    /// The write-shape idiom table to use for classification.
    idiom_table: WriteIdiomTable,
}

impl RetryIdempotencyAnalyzer {
    /// Create a new analyzer with the given idiom table.
    pub fn new(idiom_table: WriteIdiomTable) -> Self {
        RetryIdempotencyAnalyzer { idiom_table }
    }

    /// Create a new analyzer with the built-in v0.1.0 idiom table.
    pub fn with_builtin_table() -> Self {
        RetryIdempotencyAnalyzer {
            idiom_table: crate::write_idiom_table::builtin_write_idiom_table_v0_1_0(),
        }
    }

    /// Analyze a list of retry facts and return findings and diagnostics.
    ///
    /// Returns (findings, diagnostics):
    /// - Findings for non-idempotent retries (REQ-T5)
    /// - Coverage diagnostics for unknown idioms (REQ-T9)
    pub fn analyze(
        &self,
        facts: &[RetryIdempotencyFact],
    ) -> (Vec<RetryIdempotencyFinding>, Vec<RetryCoverageDiagnostic>) {
        let mut findings = Vec::new();
        let mut diagnostics = Vec::new();

        for fact in facts {
            let classification = self.idiom_table.classify(&fact.write_method);
            match classification {
                IdempotencyClass::NonIdempotent => {
                    findings.push(RetryIdempotencyFinding {
                        source_file: fact.source_file.clone(),
                        line: fact.line,
                        function: fact.function.clone(),
                        write_method: fact.write_method.clone(),
                        classification: "unsafe-retry".to_string(),
                        detail: format!(
                            "Retry of non-idempotent write '{}' in {} at {}:{}",
                            fact.write_method, fact.function, fact.source_file, fact.line
                        ),
                    });
                }
                IdempotencyClass::Unknown => {
                    diagnostics.push(RetryCoverageDiagnostic {
                        source_file: fact.source_file.clone(),
                        line: fact.line,
                        write_method: fact.write_method.clone(),
                        table_version: self.idiom_table.table_version.clone(),
                    });
                }
                IdempotencyClass::Idempotent => {
                    // No finding or diagnostic — safe retry.
                }
            }
        }

        (findings, diagnostics)
    }

    /// Get the idiom table version.
    pub fn table_version(&self) -> &str {
        &self.idiom_table.table_version
    }

    /// Get the idiom table schema version.
    pub fn schema_version(&self) -> &str {
        &self.idiom_table.schema_version
    }
}

// ---------------------------------------------------------------------------
// Conversion helper (to be used by CLI integration, not here)
// ---------------------------------------------------------------------------

// The CLI layer converts vampiro-rust-frontend::lifecycle::RetryFact into
// RetryIdempotencyFact. This crate does not depend on the frontend crate.

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fact(
        file: &str,
        line: usize,
        function: &str,
        write_method: &str,
    ) -> RetryIdempotencyFact {
        RetryIdempotencyFact {
            source_file: file.to_string(),
            line,
            function: function.to_string(),
            write_method: write_method.to_string(),
            retry_kind: "loop".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // REQ-T5: unsafe retry detection
    // -----------------------------------------------------------------------

    #[test]
    fn non_idempotent_write_produces_unsafe_retry_finding() {
        let analyzer = RetryIdempotencyAnalyzer::with_builtin_table();
        let facts = vec![make_fact("src/db.rs", 42, "save_order", "insert")];

        let (findings, _diagnostics) = analyzer.analyze(&facts);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].classification, "unsafe-retry");
        assert_eq!(findings[0].write_method, "insert");
        assert!(findings[0].detail.contains("non-idempotent"));
    }

    #[test]
    fn idempotent_write_produces_no_finding() {
        let analyzer = RetryIdempotencyAnalyzer::with_builtin_table();
        let facts = vec![make_fact("src/db.rs", 10, "set_key", "set")];

        let (findings, _diagnostics) = analyzer.analyze(&facts);
        assert!(findings.is_empty());
    }

    #[test]
    fn upsert_is_idempotent_no_finding() {
        let analyzer = RetryIdempotencyAnalyzer::with_builtin_table();
        let facts = vec![make_fact("src/db.rs", 15, "create_or_update", "upsert")];

        let (findings, _diagnostics) = analyzer.analyze(&facts);
        assert!(findings.is_empty());
    }

    #[test]
    fn patch_is_non_idempotent() {
        let analyzer = RetryIdempotencyAnalyzer::with_builtin_table();
        let facts = vec![make_fact("src/api.rs", 99, "update_user_partial", "patch")];

        let (findings, _diagnostics) = analyzer.analyze(&facts);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].classification, "unsafe-retry");
    }

    // -----------------------------------------------------------------------
    // REQ-T9: unknown idiom coverage diagnostic
    // -----------------------------------------------------------------------

    #[test]
    fn unknown_write_method_produces_coverage_diagnostic() {
        let analyzer = RetryIdempotencyAnalyzer::with_builtin_table();
        let facts = vec![make_fact("src/custom.rs", 30, "do_custom", "custom_write")];

        let (findings, diagnostics) = analyzer.analyze(&facts);
        assert!(findings.is_empty());
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].write_method, "custom_write");
        assert_eq!(diagnostics[0].table_version, "0.1.0");
    }

    // -----------------------------------------------------------------------
    // Multiple facts
    // -----------------------------------------------------------------------

    #[test]
    fn mixed_idempotent_and_non_idempotent() {
        let analyzer = RetryIdempotencyAnalyzer::with_builtin_table();
        let facts = vec![
            make_fact("src/a.rs", 10, "fn_a", "set"),    // idempotent
            make_fact("src/b.rs", 20, "fn_b", "insert"), // non-idempotent
            make_fact("src/c.rs", 30, "fn_c", "custom_unknown"), // unknown
        ];

        let (findings, diagnostics) = analyzer.analyze(&facts);
        assert_eq!(findings.len(), 1);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(findings[0].write_method, "insert");
        assert_eq!(diagnostics[0].write_method, "custom_unknown");
    }

    // -----------------------------------------------------------------------
    // Empty input
    // -----------------------------------------------------------------------

    #[test]
    fn empty_facts_produce_no_results() {
        let analyzer = RetryIdempotencyAnalyzer::with_builtin_table();
        let (findings, diagnostics) = analyzer.analyze(&[]);
        assert!(findings.is_empty());
        assert!(diagnostics.is_empty());
    }

    // -----------------------------------------------------------------------
    // Req-T6: cross-reference support
    // -----------------------------------------------------------------------

    #[test]
    fn finding_contains_sufficient_info_for_law_cross_reference() {
        // REQ-T6: findings must be cross-referenceable with law evidence.
        // The finding carries function, file, line, and write_method which
        // should be enough to correlate with a law evidence record.
        let analyzer = RetryIdempotencyAnalyzer::with_builtin_table();
        let facts = vec![make_fact("src/db.rs", 42, "save_order", "insert")];

        let (findings, _) = analyzer.analyze(&facts);
        assert_eq!(findings[0].function, "save_order");
        assert_eq!(findings[0].source_file, "src/db.rs");
        assert_eq!(findings[0].line, 42);

        // The function+file+line forms a stable cross-reference key.
        let cross_ref_key = format!(
            "{}:{}:{}",
            findings[0].source_file, findings[0].line, findings[0].function
        );
        assert_eq!(cross_ref_key, "src/db.rs:42:save_order");
    }

    // -----------------------------------------------------------------------
    // Custom table
    // -----------------------------------------------------------------------

    #[test]
    fn analyzer_with_custom_table() {
        let mut table = crate::write_idiom_table::WriteIdiomTable::new("0.1.0-test");
        table.add_entry(crate::write_idiom_table::WriteIdiomEntry::new(
            "custom_op",
            crate::write_idiom_table::IdempotencyClass::Idempotent,
            vec!["my_custom_op"],
        ));
        let analyzer = RetryIdempotencyAnalyzer::new(table);

        let facts = vec![make_fact("src/lib.rs", 5, "run", "my_custom_op")];
        let (findings, _diagnostics) = analyzer.analyze(&facts);
        assert!(findings.is_empty());
    }
}
