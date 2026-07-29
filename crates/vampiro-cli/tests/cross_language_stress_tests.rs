//! Cross-language frontend stress tests (vampiro-tmf.3).
//!
//! Validates that each language frontend can parse realistic source files
//! and produce CIR graphs without crashes. Known frontend bugs are reported
//! as warnings (not failures) and tracked in separate issues:
//!   - vampiro-y4y: Python duplicate __init__ IDs
//!   - vampiro-276: Julia duplicate <anonymous> IDs
//!   - vampiro-3hk: Clojure edges to non-extracted macros/constructors
//!   - vampiro-fhg: Python edges to non-extracted builtins
//!
//! Run: `cargo test cross_language_*`

use std::path::{Path, PathBuf};

/// Resolve the workspace root from this crate's manifest dir.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn cross_fixture(lang: &str, name: &str) -> PathBuf {
    workspace_root()
        .join("tests/fixtures/stress/cross-language")
        .join(lang)
        .join(name)
}

fn read_fixture(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Extract and validate. Reports structural issues as warnings (not failures).
fn extract_and_validate<F: vampiro_cir::Frontend>(
    frontend: &F,
    source: &str,
    path: &Path,
    label: &str,
) -> vampiro_cir::CirGraph {
    let graph = frontend
        .extract(source, path)
        .unwrap_or_else(|e| panic!("{label}: extraction panicked/crashed: {e}"));

    assert!(
        !graph.nodes.is_empty(),
        "{label}: extracted graph has no nodes"
    );

    // Check duplicate node IDs (known frontend bugs — warn, don't fail)
    let mut seen_ids = std::collections::HashSet::new();
    let mut dupes = Vec::new();
    for node in &graph.nodes {
        if !seen_ids.insert(node.id.clone()) {
            dupes.push(node.id.clone());
        }
    }
    if !dupes.is_empty() {
        eprintln!(
            "WARN {label}: {} duplicate node IDs: {:?}",
            dupes.len(),
            dupes
        );
    }

    // Check edge targets that don't exist (known frontend bugs — warn, don't fail)
    let node_ids: std::collections::HashSet<&str> =
        graph.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut dangling = Vec::new();
    for edge in &graph.edges {
        if !node_ids.contains(edge.source.as_str()) {
            dangling.push(format!("source '{}'", edge.source));
        }
        if !node_ids.contains(edge.target.as_str()) {
            dangling.push(format!("target '{}'", edge.target));
        }
    }
    if !dangling.is_empty() {
        eprintln!(
            "WARN {label}: {} dangling edge references: {:?}",
            dangling.len(),
            dangling
        );
    }

    graph
}

// ---------------------------------------------------------------------------
// Python stress tests
// ---------------------------------------------------------------------------

#[test]
fn cross_language_python_http_lib() {
    let path = cross_fixture("python", "http_lib.py");
    let source = read_fixture(&path);
    let frontend = vampiro_python_frontend::PythonFrontend;

    let graph = extract_and_validate(&frontend, &source, &path, "python/http_lib.py");
    assert!(
        graph.nodes.len() >= 5,
        "expected ≥5 function nodes (Session methods, retry, fetch_user, HTTPErrorV), got {}",
        graph.nodes.len()
    );
}

#[test]
fn cross_language_python_cli() {
    let path = cross_fixture("python", "cli.py");
    let source = read_fixture(&path);
    let frontend = vampiro_python_frontend::PythonFrontend;

    let graph = extract_and_validate(&frontend, &source, &path, "python/cli.py");
    assert!(
        graph.nodes.len() >= 4,
        "expected ≥4 function nodes (greet, add, process, run), got {}",
        graph.nodes.len()
    );
}

#[test]
fn cross_language_python_data_pipeline() {
    let path = cross_fixture("python", "data_pipeline.py");
    let source = read_fixture(&path);
    let frontend = vampiro_python_frontend::PythonFrontend;

    let graph = extract_and_validate(&frontend, &source, &path, "python/data_pipeline.py");
    assert!(
        graph.nodes.len() >= 3,
        "expected ≥3 function nodes (parse_csv, aggregate, generate_report), got {}",
        graph.nodes.len()
    );
}

// ---------------------------------------------------------------------------
// Clojure stress tests
// ---------------------------------------------------------------------------

#[test]
fn cross_language_clojure_http_server() {
    let path = cross_fixture("clojure", "http_server.clj");
    let source = read_fixture(&path);
    let frontend = vampiro_clojure_frontend::ClojureFrontend;

    let graph = extract_and_validate(&frontend, &source, &path, "clojure/http_server.clj");
    assert!(
        graph.nodes.len() >= 5,
        "expected ≥5 nodes (handler fns, router, middleware), got {}",
        graph.nodes.len()
    );
}

#[test]
fn cross_language_clojure_async_example() {
    let path = cross_fixture("clojure", "async_example.clj");
    let source = read_fixture(&path);
    let frontend = vampiro_clojure_frontend::ClojureFrontend;

    let graph = extract_and_validate(&frontend, &source, &path, "clojure/async_example.clj");
    assert!(
        graph.nodes.len() >= 3,
        "expected ≥3 nodes (producer, consumer, pipeline, -main), got {}",
        graph.nodes.len()
    );
}

#[test]
fn cross_language_clojure_data_processing() {
    let path = cross_fixture("clojure", "data_processing.clj");
    let source = read_fixture(&path);
    let frontend = vampiro_clojure_frontend::ClojureFrontend;

    let graph = extract_and_validate(&frontend, &source, &path, "clojure/data_processing.clj");
    assert!(
        graph.nodes.len() >= 3,
        "expected ≥3 nodes (parse-line, parse-csv, aggregate-by), got {}",
        graph.nodes.len()
    );
}

// ---------------------------------------------------------------------------
// Julia stress tests
// ---------------------------------------------------------------------------

#[test]
fn cross_language_julia_data_analysis() {
    let path = cross_fixture("julia", "data_analysis.jl");
    let source = read_fixture(&path);
    let frontend = vampiro_julia_frontend::JuliaFrontend;

    let graph = extract_and_validate(&frontend, &source, &path, "julia/data_analysis.jl");
    assert!(
        graph.nodes.len() >= 4,
        "expected ≥4 nodes (constructors, operations, describe, main), got {}",
        graph.nodes.len()
    );
}

#[test]
fn cross_language_julia_web_server() {
    let path = cross_fixture("julia", "web_server.jl");
    let source = read_fixture(&path);
    let frontend = vampiro_julia_frontend::JuliaFrontend;

    let graph = extract_and_validate(&frontend, &source, &path, "julia/web_server.jl");
    assert!(
        graph.nodes.len() >= 4,
        "expected ≥4 nodes (ok, not_found, middleware, handlers, serve), got {}",
        graph.nodes.len()
    );
}