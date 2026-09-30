//! E2E test for the composition tracer over a real Rust frontend extraction.
//!
//! Loads the negative fixture
//! `tests/fixtures/add-core-seam-analysis/1/composition_break.rs`, extracts a
//! CIR graph via `vampiro-rust-frontend`, runs the seam-analysis composition
//! tracer, and asserts that a spec-conformant `composition` finding is
//! produced with the required fields (REQ-7).

use std::path::Path;

use vampiro_cir::Frontend;
use vampiro_rust_frontend::RustFrontend;
use vampiro_seam_analysis::{analyze, Axis, Evidence, Severity};

/// Resolve the fixture path relative to the workspace root.
fn fixture_path() -> String {
    let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/add-core-seam-analysis/1/composition_break.rs");
    path.to_string_lossy().to_string()
}

#[test]
fn composition_e2e_negative_fixture() {
    let path = fixture_path();
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));

    let graph = RustFrontend
        .extract(
            &source,
            Path::new("tests/fixtures/add-core-seam-analysis/1/composition_break.rs"),
        )
        .expect("frontend extraction must succeed");

    // Sanity: the fixture defines the three functions and at least one call.
    assert!(
        graph.nodes.len() >= 3,
        "expected >=3 nodes, got {}",
        graph.nodes.len()
    );
    assert!(!graph.edges.is_empty(), "expected at least one call edge");

    let findings = analyze(&graph);

    // The fixture is a negative case: at least one composition finding must
    // be produced. Since vampiro-224.9 the return-boundary (codomain-vs-
    // codomain) check fires only for return-position calls — the fixture's
    // tuple tail means the decl-edge comparison is correctly suppressed, and
    // the seeded TP surfaces via the data-flow check: `parse_amount`'s
    // Option<f64> flows into `apply_discount`'s `f64` slot without being
    // unwrapped (REQ-7).
    let composition: Vec<_> = findings
        .iter()
        .filter(|f| f.axis == Axis::Composition)
        .collect();
    assert!(
        !composition.is_empty(),
        "expected at least one composition finding; got {findings:?}"
    );

    let f = composition[0];

    // Required finding fields (REQ-4, REQ-7).
    assert_eq!(f.rule, "REQ-7");
    assert_eq!(f.axis, Axis::Composition);
    assert_eq!(f.severity, Severity::Medium);
    assert!(
        f.line_range.start <= f.line_range.end,
        "line range must be well-formed"
    );

    // Side-by-side evidence (REQ-7): both caller-produced and callee-expected
    // shapes are present. The seam here is the argument slot: Option<f64>
    // produced where f64 is expected.
    #[allow(irrefutable_let_patterns)]
    let Evidence::SlotMismatch {
        slot,
        callee_expected,
        caller_produced,
    } = &f.evidence
    else {
        panic!("expected slot mismatch evidence, got {:?}", f.evidence);
    };
    assert_eq!(*slot, 0, "the break is at apply_discount's first parameter");
    assert!(
        callee_expected != caller_produced,
        "side-by-side shapes must differ in a composition finding"
    );
}
