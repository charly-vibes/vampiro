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

/// Deref coercion (vampiro-224.13): `&mut Vec<(u32,)>` passed where
/// `&[(u32,)]` is expected compiles via Deref coercion and must not produce
/// a composition-break slot mismatch (the testaruda engine.rs FP class).
/// The call is statement-position, as in the original finding.
#[test]
fn composition_e2e_deref_coercion_no_finding() {
    let source = "\
struct AscentProgram;
struct SelectionContext;

fn build_ascent_program(ctx: &SelectionContext, comp_fallback: &[(u32,)]) -> AscentProgram {
    let _ = ctx;
    AscentProgram
}

pub fn needs_fallback_pass(
    ctx: &SelectionContext,
    comp_fallback: &mut Vec<(u32,)>,
) -> AscentProgram {
    let mut prog2 = build_ascent_program(ctx, comp_fallback);
    prog2.run();
    prog2
}
";
    let graph = RustFrontend
        .extract(source, Path::new("src/engine.rs"))
        .expect("frontend extraction must succeed");
    let findings = analyze(&graph);
    let composition: Vec<_> = findings
        .iter()
        .filter(|f| f.axis == Axis::Composition)
        .collect();
    assert!(
        composition.is_empty(),
        "deref coercion must not produce composition findings; got {findings:?}"
    );
}

// --- Method-combinator shape inference (vampiro-224.6) ---

/// `as_ref` on an Option inserts the reference the callee parameter expects
/// (the dont main.rs:4839 FP: `check_evidence_uri(uri, mocks.as_ref(), ...)`
/// flagged Option<HashMap> vs &Option<HashMap>).
#[test]
fn composition_e2e_option_as_ref_no_finding() {
    let source = "\
use std::collections::HashMap;

struct MockEvidenceCheckResult;

fn check_evidence_uri(
    uri: &str,
    mocks: Option<&HashMap<String, MockEvidenceCheckResult>>,
) -> u32 {
    let _ = uri;
    let _ = mocks;
    0
}

pub fn go(mocks: Option<HashMap<String, MockEvidenceCheckResult>>) -> u32 {
    check_evidence_uri(\"u\", mocks.as_ref())
}
";
    let graph = RustFrontend
        .extract(source, Path::new("src/main.rs"))
        .expect("frontend extraction must succeed");
    let findings = analyze(&graph);
    let composition: Vec<_> = findings
        .iter()
        .filter(|f| f.axis == Axis::Composition)
        .collect();
    assert!(
        composition.is_empty(),
        "as_ref must insert the reference the callee expects; got {findings:?}"
    );
}

/// `is_some_and` returns bool, not the receiver's Option shape (the wai
/// hooks.rs FP: read_hook(...).is_some_and(|c| ...) flagged Option<string>
/// vs bool).
#[test]
fn composition_e2e_is_some_and_returns_bool() {
    let source = "\
fn take(flag: bool) -> u32 {
    let _ = flag;
    0
}

pub fn go(value: Option<String>) -> u32 {
    take(value.is_some_and(|s| !s.is_empty()))
}
";
    let graph = RustFrontend
        .extract(source, Path::new("src/hooks.rs"))
        .expect("frontend extraction must succeed");
    let findings = analyze(&graph);
    let composition: Vec<_> = findings
        .iter()
        .filter(|f| f.axis == Axis::Composition)
        .collect();
    assert!(
        composition.is_empty(),
        "is_some_and must produce bool; got {findings:?}"
    );
}

/// `unwrap_or` returns the receiver's inner type, not the receiver shape.
#[test]
fn composition_e2e_unwrap_or_returns_inner() {
    let source = "\
fn take(name: String) -> u32 {
    let _ = name;
    0
}

pub fn go(value: Option<String>) -> u32 {
    take(value.unwrap_or_else(|| \"fallback\".to_string()))
}
";
    let graph = RustFrontend
        .extract(source, Path::new("src/lib.rs"))
        .expect("frontend extraction must succeed");
    let findings = analyze(&graph);
    let composition: Vec<_> = findings
        .iter()
        .filter(|f| f.axis == Axis::Composition)
        .collect();
    assert!(
        composition.is_empty(),
        "unwrap_or_else must produce the inner type; got {findings:?}"
    );
}

/// Unknown methods must degrade to Opaque (excluded from composition-break
/// checking) instead of guessing the receiver's shape.
#[test]
fn composition_e2e_unknown_method_is_opaque_excluded() {
    let source = "\
fn take(count: u32) -> u32 {
    let _ = count;
    0
}

pub fn go(items: Vec<String>) -> u32 {
    take(items.len() as u32)
}
";
    let graph = RustFrontend
        .extract(source, Path::new("src/lib.rs"))
        .expect("frontend extraction must succeed");
    let findings = analyze(&graph);
    let composition: Vec<_> = findings
        .iter()
        .filter(|f| f.axis == Axis::Composition)
        .collect();
    assert!(
        composition.is_empty(),
        "unknown methods (len) must not guess the receiver shape; got {findings:?}"
    );
}
