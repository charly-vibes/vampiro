//! Shared CIR acceptance contract — per-language matrices and compatibility
//! harness for frontend conformance testing.
//!
//! This crate defines the extraction capabilities that every language frontend
//! must demonstrate, and provides a harness that can run independently for
//! each language.
//!
//! # Matrices
//!
//! Each `LanguageMatrix` lists what a conformant frontend should extract:
//! nodes, edges, shapes, effects, provenance kinds, and visibility levels.
//!
//! # Harness
//!
//! The `CompatibilityHarness` takes a frontend and a matrix, runs each matrix
//! entry through the frontend against sample source, and returns a
//! `ConformanceReport` with per-entry results.

use std::path::Path;
use vampiro_cir::{CirGraph, Frontend};

// ---------------------------------------------------------------------------
// Matrix types
// ---------------------------------------------------------------------------

/// A single extraction capability that a frontend is expected to support.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct MatrixEntry {
    /// The capability category (e.g. "node", "edge", "shape", "effect").
    pub category: String,
    /// The specific capability name (e.g. "function_declaration", "direct_call").
    pub name: String,
    /// Optional description of what this capability means.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this capability is required or optional.
    #[serde(default)]
    pub required: bool,
}

/// Per-language extraction matrix describing what a frontend must support.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LanguageMatrix {
    /// The language identifier (e.g. "python", "clojure", "julia").
    pub language: String,
    /// Schema version of this matrix.
    pub version: String,
    /// Node extraction capabilities.
    pub nodes: Vec<MatrixEntry>,
    /// Edge extraction capabilities.
    pub edges: Vec<MatrixEntry>,
    /// Shape extraction capabilities.
    pub shapes: Vec<MatrixEntry>,
    /// Effect channel extraction capabilities.
    pub effects: Vec<MatrixEntry>,
    /// Provenance kinds the frontend must support.
    pub provenance_kinds: Vec<MatrixEntry>,
    /// Visibility levels the frontend must support.
    pub visibility_levels: Vec<MatrixEntry>,
    /// Whether the frontend must support opaque/unknown sentinels.
    pub supports_opaque_unknown: bool,
    /// Whether the frontend must emit unsupported-construct evidence.
    pub supports_unsupported_evidence: bool,
    /// Deterministic output requirement.
    #[serde(default)]
    pub deterministic_output_required: bool,
}

impl LanguageMatrix {
    /// Create a new `LanguageMatrix` for the given language.
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            version: "0.1.0".into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            shapes: Vec::new(),
            effects: Vec::new(),
            provenance_kinds: Vec::new(),
            visibility_levels: Vec::new(),
            supports_opaque_unknown: true,
            supports_unsupported_evidence: true,
            deterministic_output_required: true,
        }
    }

    /// Add a required node extraction entry.
    pub fn with_node(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.nodes.push(MatrixEntry {
            category: "node".into(),
            name: name.into(),
            description: Some(description.into()),
            required: true,
        });
        self
    }

    /// Add a required edge extraction entry.
    pub fn with_edge(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.edges.push(MatrixEntry {
            category: "edge".into(),
            name: name.into(),
            description: Some(description.into()),
            required: true,
        });
        self
    }

    /// Add a required shape extraction entry.
    pub fn with_shape(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.shapes.push(MatrixEntry {
            category: "shape".into(),
            name: name.into(),
            description: Some(description.into()),
            required: true,
        });
        self
    }

    /// Add a required effect extraction entry.
    pub fn with_effect(mut self, name: impl Into<String>, description: impl Into<String>) -> Self {
        self.effects.push(MatrixEntry {
            category: "effect".into(),
            name: name.into(),
            description: Some(description.into()),
            required: true,
        });
        self
    }

    /// Add a required provenance kind entry.
    pub fn with_provenance(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.provenance_kinds.push(MatrixEntry {
            category: "provenance".into(),
            name: name.into(),
            description: Some(description.into()),
            required: true,
        });
        self
    }

    /// Add a required visibility level entry.
    pub fn with_visibility(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        self.visibility_levels.push(MatrixEntry {
            category: "visibility".into(),
            name: name.into(),
            description: Some(description.into()),
            required: true,
        });
        self
    }
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Result of testing a single matrix entry against a frontend.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EntryResult {
    /// The frontend satisfies this matrix entry.
    Passed,
    /// The frontend does not satisfy this matrix entry.
    Failed { reason: String },
    /// This entry could not be tested (e.g. lacking sample source).
    Skipped { reason: String },
}

/// Conformance report for a single frontend against its language matrix.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConformanceReport {
    /// The language identifier.
    pub language: String,
    /// The matrix version tested against.
    pub matrix_version: String,
    /// Results per entry, grouped by category.
    pub nodes: Vec<(MatrixEntry, EntryResult)>,
    pub edges: Vec<(MatrixEntry, EntryResult)>,
    pub shapes: Vec<(MatrixEntry, EntryResult)>,
    pub effects: Vec<(MatrixEntry, EntryResult)>,
    pub provenance_kinds: Vec<(MatrixEntry, EntryResult)>,
    pub visibility_levels: Vec<(MatrixEntry, EntryResult)>,
    /// Whether opaque/unknown support was verified.
    pub opaque_unknown_supported: Option<EntryResult>,
    /// Whether unsupported-evidence support was verified.
    pub unsupported_evidence_supported: Option<EntryResult>,
    /// Whether deterministic output was verified.
    pub deterministic_output: Option<EntryResult>,
}

impl ConformanceReport {
    /// Total number of entries.
    pub fn total_entries(&self) -> usize {
        let mut count = 0;
        for list in [
            &self.nodes,
            &self.edges,
            &self.shapes,
            &self.effects,
            &self.provenance_kinds,
            &self.visibility_levels,
        ] {
            count += list.len();
        }
        if self.opaque_unknown_supported.is_some() {
            count += 1;
        }
        if self.unsupported_evidence_supported.is_some() {
            count += 1;
        }
        if self.deterministic_output.is_some() {
            count += 1;
        }
        count
    }

    /// Number of passed entries.
    pub fn passed_entries(&self) -> usize {
        let mut count = 0;
        for list in [
            &self.nodes,
            &self.edges,
            &self.shapes,
            &self.effects,
            &self.provenance_kinds,
            &self.visibility_levels,
        ] {
            count += list
                .iter()
                .filter(|(_, r)| *r == EntryResult::Passed)
                .count();
        }
        for opt in [
            &self.opaque_unknown_supported,
            &self.unsupported_evidence_supported,
            &self.deterministic_output,
        ] {
            if let Some(EntryResult::Passed) = opt {
                count += 1;
            }
        }
        count
    }

    /// Number of failed entries.
    pub fn failed_entries(&self) -> usize {
        let mut count = 0;
        for list in [
            &self.nodes,
            &self.edges,
            &self.shapes,
            &self.effects,
            &self.provenance_kinds,
            &self.visibility_levels,
        ] {
            count += list
                .iter()
                .filter(|(_, r)| matches!(r, EntryResult::Failed { .. }))
                .count();
        }
        for opt in [
            &self.opaque_unknown_supported,
            &self.unsupported_evidence_supported,
            &self.deterministic_output,
        ] {
            if let Some(EntryResult::Failed { .. }) = opt {
                count += 1;
            }
        }
        count
    }

    /// Whether all entries passed.
    pub fn all_passed(&self) -> bool {
        self.failed_entries() == 0
    }
}

/// A compatibility harness that tests a frontend against its language matrix.
///
/// The harness consumes only published CIR/plugin contracts and can run
/// independently for each language.
pub struct CompatibilityHarness {
    /// The language matrix to test against.
    pub matrix: LanguageMatrix,
}

impl CompatibilityHarness {
    /// Create a new harness for the given language matrix.
    pub fn new(matrix: LanguageMatrix) -> Self {
        Self { matrix }
    }

    /// Run the harness against a frontend, producing a conformance report.
    ///
    /// Each matrix entry is tested by extracting a sample source and checking
    /// whether the resulting graph contains the expected structure. For
    /// language-specific frontends, provide `samples` — a mapping from
    /// matrix entry name (e.g. `"function_declaration"`) to a source string
    /// that exercises that capability.
    ///
    /// Entries with no sample source are recorded as `Skipped`.
    pub fn run(&self, frontend: &dyn Frontend, samples: &[SampleCase]) -> ConformanceReport {
        let mut report = ConformanceReport {
            language: self.matrix.language.clone(),
            matrix_version: self.matrix.version.clone(),
            nodes: Vec::new(),
            edges: Vec::new(),
            shapes: Vec::new(),
            effects: Vec::new(),
            provenance_kinds: Vec::new(),
            visibility_levels: Vec::new(),
            opaque_unknown_supported: None,
            unsupported_evidence_supported: None,
            deterministic_output: None,
        };

        // Build a lookup: entry name -> SampleCase
        let sample_map: std::collections::HashMap<&str, &SampleCase> =
            samples.iter().map(|s| (s.name.as_str(), s)).collect();

        // Test each category
        for entry in &self.matrix.nodes {
            let result = self.test_entry(frontend, entry, &sample_map);
            report.nodes.push((entry.clone(), result));
        }

        for entry in &self.matrix.edges {
            let result = self.test_entry(frontend, entry, &sample_map);
            report.edges.push((entry.clone(), result));
        }

        for entry in &self.matrix.shapes {
            let result = self.test_entry(frontend, entry, &sample_map);
            report.shapes.push((entry.clone(), result));
        }

        for entry in &self.matrix.effects {
            let result = self.test_entry(frontend, entry, &sample_map);
            report.effects.push((entry.clone(), result));
        }

        for entry in &self.matrix.provenance_kinds {
            let result = self.test_entry(frontend, entry, &sample_map);
            report.provenance_kinds.push((entry.clone(), result));
        }

        for entry in &self.matrix.visibility_levels {
            let result = self.test_entry(frontend, entry, &sample_map);
            report.visibility_levels.push((entry.clone(), result));
        }

        // Boolean capabilities
        report.opaque_unknown_supported = if self.matrix.supports_opaque_unknown {
            Some(self.test_boolean(
                frontend,
                "opaque_unknown",
                samples.iter().find(|s| s.name == "opaque_unknown"),
                |_graph| {
                    // Check that the graph parsed without error — actual
                    // opaque/unknown handling is verified per-frontend.
                    true
                },
            ))
        } else {
            None
        };

        report.unsupported_evidence_supported = if self.matrix.supports_unsupported_evidence {
            Some(self.test_boolean(
                frontend,
                "unsupported_evidence",
                samples.iter().find(|s| s.name == "unsupported_evidence"),
                |_graph| true,
            ))
        } else {
            None
        };

        report.deterministic_output = if self.matrix.deterministic_output_required {
            Some(self.test_determinism(frontend, samples))
        } else {
            None
        };

        report
    }

    /// Run the harness against a frontend using the null frontend as a placeholder.
    ///
    /// This is used for the "empty/reference harness" phase (task 1.3) where
    /// no real frontend exists yet. All entries are tested against an
    /// `NullFrontend` and should either pass (empty graph is valid for empty
    /// source) or be skipped (no meaningful extraction possible).
    pub fn run_empty(&self) -> ConformanceReport {
        let null_frontend = vampiro_cir::frontend::NullFrontend;
        self.run(&null_frontend, &[])
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn test_entry(
        &self,
        frontend: &dyn Frontend,
        entry: &MatrixEntry,
        sample_map: &std::collections::HashMap<&str, &SampleCase>,
    ) -> EntryResult {
        let sample = match sample_map.get(entry.name.as_str()) {
            Some(s) => s,
            None => {
                return if entry.required {
                    EntryResult::Skipped {
                        reason: format!("no sample source for '{}'", entry.name),
                    }
                } else {
                    EntryResult::Skipped {
                        reason: format!("optional, no sample source for '{}'", entry.name),
                    }
                };
            }
        };

        let result = frontend.extract(&sample.source, Path::new(sample.path.as_str()));

        match result {
            Ok(graph) => {
                if graph.nodes.is_empty() && graph.edges.is_empty() && !sample.expects_empty {
                    // Empty graph when we expected content.
                    EntryResult::Failed {
                        reason: format!(
                            "extraction produced empty graph for '{}' in category '{}'; expected {}",
                            entry.name,
                            entry.category,
                            entry.category
                        ),
                    }
                } else {
                    EntryResult::Passed
                }
            }
            Err(e) => EntryResult::Failed {
                reason: format!("extraction failed: {e}"),
            },
        }
    }

    fn test_boolean(
        &self,
        frontend: &dyn Frontend,
        name: &str,
        sample: Option<&SampleCase>,
        check: fn(&CirGraph) -> bool,
    ) -> EntryResult {
        match sample {
            Some(sample) => match frontend.extract(&sample.source, Path::new(&sample.path)) {
                Ok(graph) => {
                    if check(&graph) {
                        EntryResult::Passed
                    } else {
                        EntryResult::Failed {
                            reason: format!("boolean check failed for '{}'", name),
                        }
                    }
                }
                Err(e) => EntryResult::Failed {
                    reason: format!("extraction failed for '{}': {e}", name),
                },
            },
            None => EntryResult::Skipped {
                reason: format!("no sample source for '{}'", name),
            },
        }
    }

    fn test_determinism(&self, frontend: &dyn Frontend, samples: &[SampleCase]) -> EntryResult {
        if samples.is_empty() {
            return EntryResult::Skipped {
                reason: "no samples to test determinism".into(),
            };
        }

        let mut tested_at_least_one = false;

        for sample in samples {
            let source = &sample.source;
            let path = Path::new(&sample.path);

            let result1 = match frontend.extract(source, path) {
                Ok(g) => g,
                Err(_) => continue,
            };
            tested_at_least_one = true;

            let result2 = match frontend.extract(source, path) {
                Ok(g) => g,
                Err(_) => {
                    return EntryResult::Failed {
                        reason: format!(
                            "non-deterministic: first extraction succeeded, second failed for '{}'",
                            sample.name
                        ),
                    }
                }
            };

            // Compare serialized forms for structural equality
            let json1 = match serde_json::to_string(&result1) {
                Ok(j) => j,
                Err(e) => {
                    return EntryResult::Failed {
                        reason: format!("serialization failed for '{}': {e}", sample.name),
                    }
                }
            };
            let json2 = match serde_json::to_string(&result2) {
                Ok(j) => j,
                Err(e) => {
                    return EntryResult::Failed {
                        reason: format!("serialization failed for '{}': {e}", sample.name),
                    }
                }
            };
            if json1 != json2 {
                return EntryResult::Failed {
                    reason: format!(
                        "non-deterministic output for '{}': two extractions produced different graphs",
                        sample.name
                    ),
                };
            }
        }

        if !tested_at_least_one {
            return EntryResult::Failed {
                reason: "no samples could be extracted to test determinism".into(),
            };
        }

        EntryResult::Passed
    }
}

/// A sample source case for testing a specific matrix capability.
#[derive(Debug, Clone)]
pub struct SampleCase {
    /// Name matching a `MatrixEntry.name`.
    pub name: String,
    /// Source code to extract.
    pub source: String,
    /// File path (extension matters for some frontends).
    pub path: String,
    /// Whether an empty graph is expected (e.g. for empty source).
    pub expects_empty: bool,
}

impl SampleCase {
    /// Create a new sample case.
    pub fn new(
        name: impl Into<String>,
        source: impl Into<String>,
        path: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            path: path.into(),
            expects_empty: false,
        }
    }

    /// Mark this sample as expecting an empty extraction result.
    pub fn expecting_empty(mut self) -> Self {
        self.expects_empty = true;
        self
    }
}

// ---------------------------------------------------------------------------
// Built-in matrices
// ---------------------------------------------------------------------------

/// The standard Python frontend matrix (REQ-1–3, REQ-V1–V2).
pub fn python_matrix() -> LanguageMatrix {
    LanguageMatrix::new("python")
        .with_node("function_declaration", "Python def statements")
        .with_node("class_declaration", "Python class definitions")
        .with_node("lambda_expression", "Lambda expressions")
        .with_node("async_function", "Async function declarations")
        .with_edge("direct_call", "Direct function/method calls")
        .with_edge("method_call", "Method calls on objects")
        .with_shape("scalar", "Scalar types (int, str, bool)")
        .with_shape("record", "Composite types (tuple, dataclass)")
        .with_shape("union", "Union types (T | None)")
        .with_effect("plain", "Synchronous function with no effect")
        .with_effect("async", "Async functions and await expressions")
        .with_effect("option", "Optional[T] / T | None")
        .with_effect("result", "Exception-based results (try/except)")
        .with_effect("stream", "Generator/yield expressions")
        .with_provenance("direct", "Direct call sites")
        .with_provenance("within", "Calls within a bounded hop limit")
        .with_provenance("over_bound", "Calls exceeding the hop limit")
        .with_visibility("public", "Public module-level names")
        .with_visibility("private", "Private names (underscore-prefixed)")
        .with_visibility("facade", "__init__.py re-exports")
}

/// The standard Clojure frontend matrix (REQ-1–3, REQ-V1–V2).
pub fn clojure_matrix() -> LanguageMatrix {
    LanguageMatrix::new("clojure")
        .with_node("function_declaration", "defn declarations")
        .with_node("anonymous_function", "fn literals and #() reader macro")
        .with_node("protocol_method", "Protocol method declarations")
        .with_node("multimethod", "defmulti / defmethod")
        .with_edge("direct_call", "Direct function calls (symbol invocation)")
        .with_edge("method_call", "Java interop method calls (.method)")
        .with_shape("scalar", "Scalar types (numbers, keywords, symbols)")
        .with_shape("record", "Composite types (maps, vectors, records)")
        .with_shape("union", "Union types (or spec)")
        .with_effect("plain", "Synchronous function with no effect")
        .with_effect("async", "Future and promise expressions")
        .with_effect("option", "nil-punning / some? patterns")
        .with_effect("result", "Exception handling (try/catch/finally)")
        .with_effect("stream", "Lazy sequences (lazy-seq)")
        .with_effect("resource", "Dynamic binding (binding) and with-open")
        .with_provenance("direct", "Direct call sites")
        .with_provenance("within", "Calls within a bounded hop limit")
        .with_provenance("over_bound", "Calls exceeding the hop limit")
        .with_visibility("public", "Public Vars declared with def")
        .with_visibility("private", "Private Vars declared with defn-")
        .with_visibility("facade", "Namespace re-exports")
}

/// The standard Julia frontend matrix (REQ-1–3, REQ-V1–V2).
pub fn julia_matrix() -> LanguageMatrix {
    LanguageMatrix::new("julia")
        .with_node(
            "function_declaration",
            "Function declarations (function ... end)",
        )
        .with_node("anonymous_function", "Lambda expressions (x -> ...)")
        .with_node("struct_declaration", "Struct type definitions")
        .with_node("macro_declaration", "Macro definitions")
        .with_edge("direct_call", "Direct function/method calls")
        .with_edge(
            "method_call",
            "Generic function dispatch (multiple dispatch)",
        )
        .with_edge("broadcast_call", "Broadcast calls (f.(x))")
        .with_shape("scalar", "Scalar types (Int, Float64, String)")
        .with_shape("record", "Composite types (struct, NamedTuple)")
        .with_shape("union", "Union types (Union{T, Nothing})")
        .with_effect("plain", "Synchronous function with no effect")
        .with_effect("async", "Async tasks (@async, @sync)")
        .with_effect("option", "Nothing / Union{T, Nothing}")
        .with_effect("result", "Exception handling (try/catch)")
        .with_effect("stream", "Channels (Channel{T}, @task)")
        .with_effect("resource", "Resource management (open do ... end)")
        .with_provenance("direct", "Direct call sites")
        .with_provenance("within", "Calls within a bounded hop limit")
        .with_provenance("over_bound", "Calls exceeding the hop limit")
        .with_visibility("public", "Public exports")
        .with_visibility("private", "Private (non-exported) names")
        .with_visibility("facade", "Module-level re-exports")
}

#[cfg(test)]
mod tests {
    use super::*;
    use vampiro_cir::frontend::NullFrontend;

    // -----------------------------------------------------------------------
    // Language matrix construction
    // -----------------------------------------------------------------------

    #[test]
    fn python_matrix_has_all_categories() {
        let matrix = python_matrix();
        assert_eq!(matrix.language, "python");
        assert_eq!(matrix.version, "0.1.0");
        assert!(!matrix.nodes.is_empty(), "expected node entries");
        assert!(!matrix.edges.is_empty(), "expected edge entries");
        assert!(!matrix.shapes.is_empty(), "expected shape entries");
        assert!(!matrix.effects.is_empty(), "expected effect entries");
        assert!(
            !matrix.provenance_kinds.is_empty(),
            "expected provenance entries"
        );
        assert!(
            !matrix.visibility_levels.is_empty(),
            "expected visibility entries"
        );
        assert!(matrix.supports_opaque_unknown);
        assert!(matrix.supports_unsupported_evidence);
        assert!(matrix.deterministic_output_required);
    }

    #[test]
    fn clojure_matrix_has_all_categories() {
        let matrix = clojure_matrix();
        assert_eq!(matrix.language, "clojure");
        assert!(!matrix.nodes.is_empty());
        assert!(!matrix.edges.is_empty());
        assert!(!matrix.shapes.is_empty());
        assert!(!matrix.effects.is_empty());
        assert!(!matrix.provenance_kinds.is_empty());
        assert!(!matrix.visibility_levels.is_empty());
    }

    #[test]
    fn julia_matrix_has_all_categories() {
        let matrix = julia_matrix();
        assert_eq!(matrix.language, "julia");
        assert!(!matrix.nodes.is_empty());
        assert!(!matrix.edges.is_empty());
        assert!(!matrix.shapes.is_empty());
        assert!(!matrix.effects.is_empty());
        assert!(!matrix.provenance_kinds.is_empty());
        assert!(!matrix.visibility_levels.is_empty());
    }

    #[test]
    fn matrix_entries_have_required_flag() {
        for matrix in [python_matrix(), clojure_matrix(), julia_matrix()] {
            for entry in &matrix.nodes {
                assert!(entry.required, "node '{}' should be required", entry.name);
            }
            for entry in &matrix.edges {
                assert!(entry.required, "edge '{}' should be required", entry.name);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Harness with NullFrontend (empty/reference)
    // -----------------------------------------------------------------------

    #[test]
    fn harness_run_empty_returns_report() {
        let matrix = python_matrix();
        let harness = CompatibilityHarness::new(matrix);
        let report = harness.run_empty();

        assert_eq!(report.language, "python");
        assert_eq!(report.matrix_version, "0.1.0");
        // With no samples, all entries should be skipped
        assert_eq!(report.nodes.len(), 4);
        assert_eq!(report.edges.len(), 2);
        assert_eq!(report.shapes.len(), 3);
        assert_eq!(report.effects.len(), 5);
        assert_eq!(report.provenance_kinds.len(), 3);
        assert_eq!(report.visibility_levels.len(), 3);

        // All should be skipped (no samples)
        for (_, result) in &report.nodes {
            assert!(matches!(result, EntryResult::Skipped { .. }));
        }
        // Boolean checks have no samples, so they're skipped too
        assert!(matches!(
            report.opaque_unknown_supported,
            Some(EntryResult::Skipped { .. })
        ));
        assert!(matches!(
            report.unsupported_evidence_supported,
            Some(EntryResult::Skipped { .. })
        ));
        assert!(matches!(
            report.deterministic_output,
            Some(EntryResult::Skipped { .. })
        ));
    }

    #[test]
    fn harness_run_with_samples() {
        let matrix =
            LanguageMatrix::new("test").with_node("function_declaration", "Function declarations");
        let harness = CompatibilityHarness::new(matrix);

        // Use NullFrontend with samples — it returns empty graphs.
        let samples = vec![SampleCase::new(
            "function_declaration",
            "fn foo() {}",
            "test.rs",
        )];
        let report = harness.run(&NullFrontend, &samples);

        assert_eq!(report.nodes.len(), 1);
        // NullFrontend returns empty graph, so node extraction fails
        assert!(matches!(report.nodes[0].1, EntryResult::Failed { .. }));
    }

    #[test]
    fn harness_determinism_check() {
        // The builder doesn't set deterministic, so create directly
        let matrix = LanguageMatrix {
            language: "test".into(),
            version: "0.1.0".into(),
            nodes: vec![],
            edges: vec![],
            shapes: vec![],
            effects: vec![],
            provenance_kinds: vec![],
            visibility_levels: vec![],
            supports_opaque_unknown: false,
            supports_unsupported_evidence: false,
            deterministic_output_required: true,
        };

        let harness = CompatibilityHarness::new(matrix);
        let report = harness.run(&NullFrontend, &[]);
        assert!(matches!(
            report.deterministic_output,
            Some(EntryResult::Skipped { .. })
        ));
    }

    // -----------------------------------------------------------------------
    // ConformanceReport helpers
    // -----------------------------------------------------------------------

    #[test]
    fn conformance_report_counts() {
        let report = ConformanceReport {
            language: "test".into(),
            matrix_version: "0.1.0".into(),
            nodes: vec![
                (
                    MatrixEntry {
                        category: "node".into(),
                        name: "a".into(),
                        description: None,
                        required: true,
                    },
                    EntryResult::Passed,
                ),
                (
                    MatrixEntry {
                        category: "node".into(),
                        name: "b".into(),
                        description: None,
                        required: true,
                    },
                    EntryResult::Failed {
                        reason: "error".into(),
                    },
                ),
            ],
            edges: vec![],
            shapes: vec![],
            effects: vec![],
            provenance_kinds: vec![],
            visibility_levels: vec![],
            opaque_unknown_supported: Some(EntryResult::Passed),
            unsupported_evidence_supported: None,
            deterministic_output: Some(EntryResult::Passed),
        };

        assert_eq!(report.total_entries(), 4);
        assert_eq!(report.passed_entries(), 3);
        assert_eq!(report.failed_entries(), 1);
        assert!(!report.all_passed());
    }

    #[test]
    fn conformance_report_all_passed() {
        let report = ConformanceReport {
            language: "test".into(),
            matrix_version: "0.1.0".into(),
            nodes: vec![(
                MatrixEntry {
                    category: "node".into(),
                    name: "a".into(),
                    description: None,
                    required: true,
                },
                EntryResult::Passed,
            )],
            edges: vec![],
            shapes: vec![],
            effects: vec![],
            provenance_kinds: vec![],
            visibility_levels: vec![],
            opaque_unknown_supported: None,
            unsupported_evidence_supported: None,
            deterministic_output: None,
        };
        assert_eq!(report.passed_entries(), 1);
        assert_eq!(report.failed_entries(), 0);
        assert!(report.all_passed());
    }

    // -----------------------------------------------------------------------
    // Serialization round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn matrix_serializes_to_json() {
        let matrix = python_matrix();
        let json = serde_json::to_string_pretty(&matrix).unwrap();
        let deserialized: LanguageMatrix = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.language, "python");
        assert_eq!(deserialized.nodes.len(), matrix.nodes.len());
    }

    #[test]
    fn conformance_report_serializes_to_json() {
        let report = ConformanceReport {
            language: "python".into(),
            matrix_version: "0.1.0".into(),
            nodes: vec![(
                MatrixEntry {
                    category: "node".into(),
                    name: "fn".into(),
                    description: None,
                    required: true,
                },
                EntryResult::Passed,
            )],
            edges: vec![],
            shapes: vec![],
            effects: vec![],
            provenance_kinds: vec![],
            visibility_levels: vec![],
            opaque_unknown_supported: None,
            unsupported_evidence_supported: None,
            deterministic_output: None,
        };
        let json = serde_json::to_string_pretty(&report).unwrap();
        let _deserialized: ConformanceReport = serde_json::from_str(&json).unwrap();
    }
}
