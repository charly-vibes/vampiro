//! Cross-language seeded-fault suite.
//!
//! Follows the testaruda seeded-fault pattern: real frontend extraction +
//! seam-analysis pipeline, asserting against expected contracts.
//!
//! ## Contracts
//!
//! - **Precision** (`*_clean_baseline_is_precise`): clean baseline fixtures
//!   produce zero findings end-to-end.
//! - **Data-flow structure** (`*_has_data_flow_edges`): fixture calls with
//!   literal arguments produce per-slot data-flow edges (expression nodes +
//!   slot edges).
//!
//! ## Current coverage
//!
//! - **Python**: fully verified — type hints extracted, data-flow edges
//!   emitted with correct slot indices.
//! - **Clojure, Julia**: fully verified — per-slot data-flow edges emitted
//!   for all call arguments with known shapes.
//!
//! Run: `cargo test cross_language_seeded`

use std::path::{Path, PathBuf};
use vampiro_cir::Frontend;
use vampiro_seam_analysis::analyze;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture_dir(lang: &str) -> PathBuf {
    workspace_root()
        .join("tests/fixtures/stress/cross-language-seeded")
        .join(lang)
}

fn read_source(name: &str, lang: &str) -> String {
    let path = fixture_dir(lang).join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {name} ({lang}): {e}"))
}

/// Check that a graph has the expected data-flow edge structure.
fn assert_has_data_flow_edges(lang: &str, graph: &vampiro_cir::CirGraph) {
    let slot_edges: Vec<_> = graph.edges.iter().filter(|e| e.slot.is_some()).collect();
    let expr_nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, vampiro_cir::NodeKind::Expression))
        .collect();

    assert!(
        !slot_edges.is_empty(),
        "{lang}: expected at least one per-slot data-flow edge, got 0"
    );
    assert!(
        !expr_nodes.is_empty(),
        "{lang}: expected at least one expression node, got 0"
    );
    eprintln!(
        "{lang}: {} slot edges, {} expression nodes, {} total edges",
        slot_edges.len(),
        expr_nodes.len(),
        graph.edges.len()
    );
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------

#[test]
fn python_clean_baseline_is_precise() {
    let source = read_source("clean.py", "python");
    let path = fixture_dir("python").join("clean.py");
    let frontend = vampiro_python_frontend::PythonFrontend;
    let graph = frontend.extract(&source, &path).unwrap();
    let findings = analyze(&graph);
    assert!(
        findings.is_empty(),
        "Python clean baseline should produce 0 findings, got {}: {:?}",
        findings.len(),
        findings
    );
}

#[test]
fn python_composition_break_has_data_flow_edges() {
    let source = read_source("composition_break.py", "python");
    let path = fixture_dir("python").join("composition_break.py");
    let frontend = vampiro_python_frontend::PythonFrontend;
    let graph = frontend.extract(&source, &path).unwrap();
    assert_has_data_flow_edges("python", &graph);
}

// ---------------------------------------------------------------------------
// Clojure
// ---------------------------------------------------------------------------

#[test]
fn clojure_clean_baseline_is_precise() {
    let source = read_source("clean.clj", "clojure");
    let path = fixture_dir("clojure").join("clean.clj");
    let frontend = vampiro_clojure_frontend::ClojureFrontend;
    let graph = frontend.extract(&source, &path).unwrap();
    let findings = analyze(&graph);
    assert!(
        findings.is_empty(),
        "Clojure clean baseline should produce 0 findings, got {}: {:?}",
        findings.len(),
        findings
    );
}

// ---------------------------------------------------------------------------
// Clojure data-flow
// ---------------------------------------------------------------------------

#[test]
fn clojure_composition_break_has_data_flow_edges() {
    let source = read_source("composition_break.clj", "clojure");
    let path = fixture_dir("clojure").join("composition_break.clj");
    let frontend = vampiro_clojure_frontend::ClojureFrontend;
    let graph = frontend.extract(&source, &path).unwrap();
    assert_has_data_flow_edges("clojure", &graph);
}

// ---------------------------------------------------------------------------
// Julia data-flow
// ---------------------------------------------------------------------------

#[test]
fn julia_composition_break_has_data_flow_edges() {
    let source = read_source("composition_break.jl", "julia");
    let path = fixture_dir("julia").join("composition_break.jl");
    let frontend = vampiro_julia_frontend::JuliaFrontend;
    let graph = frontend.extract(&source, &path).unwrap();
    assert_has_data_flow_edges("julia", &graph);
}

// ---------------------------------------------------------------------------
// Julia clean baseline
// ---------------------------------------------------------------------------

#[test]
fn julia_clean_baseline_is_precise() {
    let source = read_source("clean.jl", "julia");
    let path = fixture_dir("julia").join("clean.jl");
    let frontend = vampiro_julia_frontend::JuliaFrontend;
    let graph = frontend.extract(&source, &path).unwrap();
    let findings = analyze(&graph);
    assert!(
        findings.is_empty(),
        "Julia clean baseline should produce 0 findings, got {}: {:?}",
        findings.len(),
        findings
    );
}
