//! Seeded-fault soundness + precision suite (vampiro-tmf.1).
//!
//! Follows the testaruda seeded-fault pattern: hand-built CIR graphs pinning
//! exact defect structures, each asserted against an expected-findings JSON
//! contract. This mirrors `testaruda/tests/seeded_fault.rs`, which constructs
//! its Store graph programmatically to guarantee the precise fault shape.
//!
//! ## Why hand-built graphs?
//!
//! Three of the four defect classes cannot be reliably produced via single-file
//! Rust frontend extraction today:
//! - **swallowed effect**: the frontend does not yet classify edges as
//!   `Swallowed` (discard detection is a separate enhancement);
//! - **redundancy mismatch**: a consumer with >=2 inbound edges from
//!   differently-shaped sources is not natural in single-file source;
//! - **over-exposure**: REQ-V4 requires a cross-file caller (vampiro-6ty),
//!   which the single-file frontend cannot represent.
//!
//! The composition defect and the clean baseline DO run through the real
//! frontend → analyzer pipeline.
//!
//! ## Contracts
//!
//! - **Soundness** (`fixtures_are_sound`): every seeded defect is detected,
//!   and each defect fixture produces exactly the pinned finding set — no
//!   more, no less.
//! - **Precision** (`fixtures_are_precise`): the clean baseline produces zero
//!   findings end-to-end through the real frontend.
//!
//! Run: `cargo test -p vampiro-seam-analysis --test stress_seeded_fixtures`

use std::path::{Path, PathBuf};

use vampiro_cir::{
    BoundaryKind, CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, LatticeLevel,
    NodeKind, Provenance, ScalarKind, Shape, SourceSpan, StableId, VisibilityFact, VisibilityFacts,
};
use vampiro_rust_frontend::RustFrontend;
use vampiro_seam_analysis::{analyze_with_visibility, Finding};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sid(s: &str) -> StableId {
    StableId::new(s)
}

fn span(file: &str, line: usize) -> SourceSpan {
    SourceSpan {
        file: file.into(),
        start_line: line,
        start_column: 1,
        end_line: line,
        end_column: 10,
    }
}

fn node(id: &str, file: &str, line: usize, domain: Shape, codomain: Shape) -> CirNode {
    CirNode {
        id: sid(id),
        domain,
        codomain,
        effect: EffectChannel::Plain,
        span: span(file, line),
        name: Some(id.into()),
        trust_provenance: Default::default(),
        is_test: false,
        kind: NodeKind::Declaration,
        containing_function: None,
    }
}

fn edge(id: &str, source: &str, target: &str, file: &str, line: usize) -> CirEdge {
    CirEdge {
        id: sid(id),
        source: sid(source),
        target: sid(target),
        resolution: EffectResolution::Propagated,
        unwrap_evidence: None,
        provenance: Provenance::Direct,
        span: span(file, line),
        discard_spans: Vec::new(),
        trust_provenance: Default::default(),
        slot: None,
        arg_shape: None,
    }
}

/// Resolve a fixture path under `tests/fixtures/stress/` relative to the
/// workspace root.
fn fixture_path(name: &str) -> PathBuf {
    let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/stress")
        .join(name)
}

/// A stable 4-tuple identifying a finding independently of line numbers or
/// stable IDs. Used to compare actual against expected findings.
type Signature = (String, String, String, String);

fn signature(f: &Finding) -> Signature {
    (
        f.rule.clone(),
        f.axis.to_string(),
        f.classification.clone(),
        f.severity.to_string(),
    )
}

/// The expected-findings JSON contract (a focused subset of the full
/// [`Finding`] schema, stable across line-number and stable-ID drift).
#[derive(serde::Deserialize)]
struct ExpectedContract {
    fixture: String,
    expected_findings: Vec<ExpectedFinding>,
}

#[derive(serde::Deserialize)]
struct ExpectedFinding {
    rule: String,
    axis: String,
    classification: String,
    severity: String,
}

impl ExpectedFinding {
    fn signature(&self) -> Signature {
        (
            self.rule.clone(),
            self.axis.clone(),
            self.classification.clone(),
            self.severity.clone(),
        )
    }
}

/// Load and parse an expected-findings JSON contract.
fn load_expected(fixture: &str) -> (String, Vec<Signature>) {
    let path = fixture_path(&format!("{fixture}.expected.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let contract: ExpectedContract = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("invalid JSON in {}: {e}", path.display()));
    let sigs = contract
        .expected_findings
        .iter()
        .map(|f| f.signature())
        .collect();
    (contract.fixture, sigs)
}

/// Compare actual vs expected finding signatures as sorted multisets.
fn assert_findings_match(fixture: &str, actual: &[Finding], expected: &[Signature]) {
    let mut actual_sigs: Vec<Signature> = actual.iter().map(signature).collect();
    let mut expected_sigs: Vec<Signature> = expected.to_vec();
    actual_sigs.sort();
    expected_sigs.sort();

    assert_eq!(
        actual_sigs, expected_sigs,
        "\n[fixture: {fixture}] finding set mismatch.\n\
         expected: {expected_sigs:#?}\n\
         actual:   {actual_sigs:#?}\n\
         full actual findings: {actual:#?}",
    );
}

// ---------------------------------------------------------------------------
// Seeded CIR graphs
// ---------------------------------------------------------------------------

/// Composition break (REQ-7): `aggregate` returns `Record[Scalar, Scalar]`
/// but calls `source_value` returning `Parameterized{Option, [Scalar]}`.
fn composition_graph() -> CirGraph {
    let file = "tests/fixtures/stress/composition.rs";
    let mut g = CirGraph::new(file);
    g.add_node(node(
        "aggregate",
        file,
        7,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Record(vec![
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ]),
    ));
    g.add_node(node(
        "source_value",
        file,
        3,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Parameterized {
            base: "Option".into(),
            parameters: vec![Shape::Scalar(ScalarKind::Unit)],
        },
    ));
    g.add_edge(edge("e1", "aggregate", "source_value", file, 8));
    g
}

/// Swallowed effect (REQ-9): `report` discards the `Result` channel from
/// `lookup` via `Swallowed` resolution.
fn swallowed_graph() -> CirGraph {
    let file = "tests/fixtures/stress/swallowed_effect.rs";
    let mut g = CirGraph::new(file);
    let mut callee = node(
        "lookup",
        file,
        3,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Scalar(ScalarKind::Unit),
    );
    callee.effect = EffectChannel::Result;
    g.add_node(node(
        "report",
        file,
        7,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Scalar(ScalarKind::Unit),
    ));
    g.add_node(callee);
    let mut e = edge("e1", "report", "lookup", file, 8);
    e.resolution = EffectResolution::Swallowed;
    g.add_edge(e);
    g
}

/// Redundancy mismatch (REQ-11): `use_data` receives data from two branches
/// with different codomain shapes and no adapter reconciles them. The coarse
/// composition tracer additionally flags each mismatched branch edge (REQ-7).
fn redundancy_graph() -> CirGraph {
    let file = "tests/fixtures/stress/redundancy.rs";
    let mut g = CirGraph::new(file);
    g.add_node(node(
        "primary_source_fetch",
        file,
        3,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Record(vec![
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ]),
    ));
    g.add_node(node(
        "cache_get",
        file,
        8,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Parameterized {
            base: "Option".into(),
            parameters: vec![Shape::Scalar(ScalarKind::Unit)],
        },
    ));
    g.add_node(node(
        "use_data",
        file,
        12,
        Shape::Record(vec![
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ]),
        Shape::Scalar(ScalarKind::Unit),
    ));
    g.add_edge(edge("e1", "primary_source_fetch", "use_data", file, 5));
    g.add_edge(edge("e2", "cache_get", "use_data", file, 9));
    g
}

/// Over-exposure (REQ-V4): `_internal` is a `#[doc(hidden)] pub fn` at L3
/// EnforcedOpen, internal-by-convention, reachable from a cross-file caller.
fn over_exposure_graph() -> (CirGraph, VisibilityFacts) {
    let exposed_file = "tests/fixtures/stress/over_exposure.rs";
    let caller_file = "tests/fixtures/stress/over_exposure_caller.rs";

    let mut g = CirGraph::new(exposed_file);
    // The over-exposed declaration.
    g.add_node(node(
        "_internal",
        exposed_file,
        8,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Scalar(ScalarKind::Unit),
    ));
    // A cross-file caller (different span.file) — required by vampiro-6ty.
    g.add_node(node(
        "caller",
        caller_file,
        1,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Scalar(ScalarKind::Unit),
    ));
    let mut e = edge("e1", "caller", "_internal", caller_file, 2);
    // Both codomains are Scalar → the composition tracer skips the caller
    // (void-guard), so no composition finding contaminates this fixture.
    e.resolution = EffectResolution::Propagated;
    g.add_edge(e);

    let mut vis = VisibilityFacts::new("0.1.0");
    vis.add_fact(VisibilityFact {
        node: sid("_internal"),
        level: LatticeLevel::L3,
        boundary: BoundaryKind::EnforcedOpen,
        scope: "pkg".into(),
        internal_by_convention: true,
    });
    vis.add_fact(VisibilityFact {
        node: sid("caller"),
        level: LatticeLevel::L2,
        boundary: BoundaryKind::Enforced,
        scope: "other".into(),
        internal_by_convention: false,
    });
    vis.add_nesting("other", "pkg");
    // No facade re-export is declared, so REQ-V7 facade-leak cannot fire —
    // this fixture is pure over-exposure (REQ-V4).
    (g, vis)
}

/// Run the real frontend + analyzer on a fixture file, returning findings.
fn analyze_source_file(fixture_name: &str) -> Vec<Finding> {
    let path = fixture_path(&format!("{fixture_name}.rs"));
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    let out = RustFrontend
        .extract_full(&source, &path)
        .unwrap_or_else(|e| panic!("frontend extraction of {fixture_name} failed: {e}"));
    let vis = vampiro_rust_frontend::visibility_adapter::to_visibility_facts(&out);
    let (findings, _diags) = analyze_with_visibility(&out.graph, &vis);
    findings
}

// ---------------------------------------------------------------------------
// Soundness: every seeded defect is detected, with exactly the pinned set.
// ---------------------------------------------------------------------------

#[test]
fn fixtures_are_sound() {
    // --- composition ---
    {
        let g = composition_graph();
        let vis = VisibilityFacts::default();
        let (findings, _diags) = analyze_with_visibility(&g, &vis);
        let (name, expected) = load_expected("composition");
        assert_eq!(name, "composition");
        assert_findings_match("composition", &findings, &expected);
    }

    // --- swallowed effect ---
    {
        let g = swallowed_graph();
        let vis = VisibilityFacts::default();
        let (findings, _diags) = analyze_with_visibility(&g, &vis);
        let (name, expected) = load_expected("swallowed_effect");
        assert_eq!(name, "swallowed_effect");
        assert_findings_match("swallowed_effect", &findings, &expected);
    }

    // --- redundancy ---
    {
        let g = redundancy_graph();
        let vis = VisibilityFacts::default();
        let (findings, _diags) = analyze_with_visibility(&g, &vis);
        let (name, expected) = load_expected("redundancy");
        assert_eq!(name, "redundancy");
        assert_findings_match("redundancy", &findings, &expected);
    }

    // --- over-exposure ---
    {
        let (g, vis) = over_exposure_graph();
        let (findings, _diags) = analyze_with_visibility(&g, &vis);
        let (name, expected) = load_expected("over_exposure");
        assert_eq!(name, "over_exposure");
        assert_findings_match("over_exposure", &findings, &expected);
    }

    // --- data-flow seam (slot-boundary check, vampiro-yvx) ---
    {
        let findings = analyze_source_file("data_flow_seam");
        let (name, expected) = load_expected("data_flow_seam");
        assert_eq!(name, "data_flow_seam");
        assert_findings_match("data_flow_seam", &findings, &expected);
    }
}

// ---------------------------------------------------------------------------
// Precision: the clean baseline yields zero findings end-to-end.
// ---------------------------------------------------------------------------

#[test]
fn fixtures_are_precise() {
    let path = fixture_path("clean.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    let out = RustFrontend
        .extract_full(&source, &path)
        .expect("frontend extraction must succeed on the clean baseline");

    let vis = vampiro_rust_frontend::visibility_adapter::to_visibility_facts(&out);
    let (findings, diags) = analyze_with_visibility(&out.graph, &vis);

    assert!(
        findings.is_empty(),
        "clean baseline must produce zero findings; got: {findings:#?}"
    );
    assert!(
        diags.is_empty(),
        "clean baseline must produce zero diagnostics; got: {diags:#?}"
    );

    // --- data-flow seam clean baseline ---
    // Now produces one REQ-9 finding for the true discard of parse_amount's
    // Option effect via `let _ = parse_amount(input);`.
    {
        let findings = analyze_source_file("data_flow_seam_clean");
        let (name, expected) = load_expected("data_flow_seam_clean");
        assert_eq!(name, "data_flow_seam_clean");
        assert_findings_match("data_flow_seam_clean", &findings, &expected);
    }
}
