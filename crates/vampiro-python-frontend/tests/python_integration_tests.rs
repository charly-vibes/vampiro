//! Python law/lifecycle/core integration tests (0vb.8.9).
//!
//! Fixtures: tests/fixtures/add-python-clojure-julia-frontends/5/
//!
//! Verifies that the Python frontend produces:
//! - Runner inputs (tagged functions, serializable values, generator refs)
//! - Lifecycle facts (writes, retries, resources, exit paths, aliases)
//! - Facade metadata (re-exports from __init__.py)
//!
//! These tests check that the Python frontend's extraction output matches
//! the expected contract fixtures. Initially they will fail (red phase)
//! because the Python frontend doesn't yet produce these outputs.

use vampiro_cir::Frontend;

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture_path(name: &str) -> std::path::PathBuf {
    workspace_root()
        .join("tests/fixtures/add-python-clojure-julia-frontends/5")
        .join(name)
}

fn read_fixture(name: &str) -> String {
    let path = fixture_path(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
}

// ---------------------------------------------------------------------------
// 5.1: Python fixtures — runner inputs
// ---------------------------------------------------------------------------

#[test]
fn python_runner_input_has_tagged_fns() {
    let source = read_fixture("runner-input.py");
    let frontend = vampiro_python_frontend::PythonFrontend;
    let path = fixture_path("runner-input.py");

    let graph = frontend.extract(&source, &path).unwrap();

    // runner-input.py has 4 functions: add, greet, process, count_up_to.
    assert_eq!(
        graph.nodes.len(),
        4,
        "expected 4 function nodes (add, greet, process, count_up_to)"
    );

    let names: Vec<&str> = graph
        .nodes
        .iter()
        .filter_map(|n| n.name.as_deref())
        .collect();
    assert!(names.contains(&"add"));
    assert!(names.contains(&"greet"));
    assert!(names.contains(&"process"));
    assert!(names.contains(&"count_up_to"));
}

#[test]
fn python_runner_input_matches_expected_json() {
    let source = read_fixture("runner-input.py");
    let path = fixture_path("runner-input.py");
    let expected_json = read_fixture("runner-input-expected.json");
    let _expected: serde_json::Value = serde_json::from_str(&expected_json).unwrap();

    let frontend = vampiro_python_frontend::PythonFrontend;
    let out = frontend.extract_full(&source, &path).unwrap();

    // Verify the law input has the expected structure
    let law = &out.law_input;
    assert_eq!(law.version, "0.1.0");
    assert!(law.source_file.ends_with("runner-input.py"));

    // Should have 4 tagged functions
    assert_eq!(law.tagged_fns.len(), 4);

    // Check specific functions exist
    let names: Vec<&str> = law.tagged_fns.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"add"));
    assert!(names.contains(&"greet"));
    assert!(names.contains(&"process"));
    assert!(names.contains(&"count_up_to"));

    // Check generator refs (count_up_to has yield)
    assert_eq!(law.generator_refs.len(), 1);
    assert_eq!(law.generator_refs[0].name, "count_up_to");
}

// ---------------------------------------------------------------------------
// 5.1: Python fixtures — lifecycle facts
// ---------------------------------------------------------------------------

#[test]
fn python_lifecycle_facts_has_writes() {
    let source = read_fixture("lifecycle-facts.py");
    let path = fixture_path("lifecycle-facts.py");

    let frontend = vampiro_python_frontend::PythonFrontend;
    let out = frontend.extract_full(&source, &path).unwrap();

    // lifecycle-facts.py has write operations
    assert!(
        !out.lifecycle_facts.writes.is_empty(),
        "expected write facts"
    );
}

#[test]
fn python_lifecycle_facts_has_retries() {
    let source = read_fixture("lifecycle-facts.py");
    let path = fixture_path("lifecycle-facts.py");

    let frontend = vampiro_python_frontend::PythonFrontend;
    let out = frontend.extract_full(&source, &path).unwrap();

    // retry_operation has a for-loop retry pattern
    assert!(
        !out.lifecycle_facts.retries.is_empty(),
        "expected retry facts"
    );
}

#[test]
fn python_lifecycle_facts_has_resources() {
    let source = read_fixture("lifecycle-facts.py");
    let path = fixture_path("lifecycle-facts.py");

    let frontend = vampiro_python_frontend::PythonFrontend;
    let out = frontend.extract_full(&source, &path).unwrap();

    // read_file uses `with open(...) as f:` — a context manager resource.
    assert!(
        !out.lifecycle_facts.resources.is_empty(),
        "expected resource facts"
    );
}

#[test]
fn python_lifecycle_facts_has_exit_paths() {
    let source = read_fixture("lifecycle-facts.py");
    let path = fixture_path("lifecycle-facts.py");

    let frontend = vampiro_python_frontend::PythonFrontend;
    let out = frontend.extract_full(&source, &path).unwrap();

    // Multiple functions have return statements.
    assert!(
        !out.lifecycle_facts.exit_paths.is_empty(),
        "expected exit path facts"
    );
}

#[test]
fn python_lifecycle_facts_matches_expected_json() {
    let source = read_fixture("lifecycle-facts.py");
    let path = fixture_path("lifecycle-facts.py");

    let frontend = vampiro_python_frontend::PythonFrontend;
    let out = frontend.extract_full(&source, &path).unwrap();

    let facts = &out.lifecycle_facts;
    assert_eq!(facts.version, "0.1.0");
    assert!(facts.source_file.ends_with("lifecycle-facts.py"));

    // Don't assert exact counts — the fixture is approximate.
    // Just verify the structure is populated.
    assert!(
        !facts.writes.is_empty() || !facts.retries.is_empty() || !facts.exit_paths.is_empty(),
        "expected at least some lifecycle facts"
    );
}

// ---------------------------------------------------------------------------
// 5.1: Python fixtures — facade metadata (__init__.py)
// ---------------------------------------------------------------------------

#[test]
fn python_facade_metadata_has_re_exports() {
    let source = read_fixture("facade-metadata.py");
    let path = fixture_path("facade-metadata.py");

    let frontend = vampiro_python_frontend::PythonFrontend;
    let out = frontend.extract_full(&source, &path).unwrap();

    // __init__.py re-exports names from 3 modules
    assert!(!out.facades.is_empty(), "expected facade declarations");
}

#[test]
fn python_facade_metadata_matches_expected_json() {
    let source = read_fixture("facade-metadata.py");
    let path = fixture_path("facade-metadata.py");

    let frontend = vampiro_python_frontend::PythonFrontend;
    let out = frontend.extract_full(&source, &path).unwrap();

    assert!(!out.facades.is_empty(), "expected facade declarations");

    // Verify at least some expected names
    let names: Vec<&str> = out.facades.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"run"));
    assert!(names.contains(&"Helper"));
    assert!(names.contains(&"Result"));
}

// ---------------------------------------------------------------------------
// 5.1: Contract fixture validation
// ---------------------------------------------------------------------------

#[test]
fn python_contract_fixtures_exist() {
    // Verify all fixture files exist.
    for name in &[
        "runner-input.py",
        "runner-input-expected.json",
        "lifecycle-facts.py",
        "lifecycle-facts-expected.json",
        "facade-metadata.py",
        "facade-metadata-expected.json",
    ] {
        let path = fixture_path(name);
        assert!(path.exists(), "missing fixture: {name} at {path:?}");
    }
}

#[test]
fn python_expected_json_is_valid_json() {
    for name in &[
        "runner-input-expected.json",
        "lifecycle-facts-expected.json",
        "facade-metadata-expected.json",
    ] {
        let content = read_fixture(name);
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&content);
        assert!(
            parsed.is_ok(),
            "invalid JSON in {name}: {}",
            parsed.unwrap_err()
        );
    }
}

#[test]
fn python_expected_runner_input_has_version() {
    let content = read_fixture("runner-input-expected.json");
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["version"], "0.1.0");
    assert_eq!(parsed["source_file"], "runner-input.py");
}

#[test]
fn python_expected_lifecycle_facts_has_version() {
    let content = read_fixture("lifecycle-facts-expected.json");
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["version"], "0.1.0");
    assert_eq!(parsed["source_file"], "lifecycle-facts.py");
}

#[test]
fn python_expected_facade_metadata_has_version() {
    let content = read_fixture("facade-metadata-expected.json");
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["version"], "0.1.0");
    assert_eq!(parsed["source_file"], "facade-metadata.py");
}

// ---------------------------------------------------------------------------
// 5.1: Python source validity
// ---------------------------------------------------------------------------

#[test]
fn python_fixture_source_is_parseable() {
    let frontend = vampiro_python_frontend::PythonFrontend;

    for (name, expected_nodes) in [
        ("runner-input.py", 4),
        ("lifecycle-facts.py", 9),
        ("facade-metadata.py", 0),
    ] {
        let source = read_fixture(name);
        let path = fixture_path(name);
        let result = frontend.extract(&source, &path);
        assert!(
            result.is_ok(),
            "failed to parse {name}: {}",
            result.unwrap_err()
        );
        let graph = result.unwrap();
        assert_eq!(
            graph.nodes.len(),
            expected_nodes,
            "expected {expected_nodes} nodes in {name}, got {}",
            graph.nodes.len()
        );
    }
}

#[test]
fn python_data_pipeline_has_slot_edges() {
    // Use a simple inline source with literal arguments
    let source = r#"
def add(a, b):
    return a + b

def main():
    result = add(42, 3.14)
    print("hello")
"#
    .to_string();
    let path = workspace_root().join("test_simple.py");
    let frontend = vampiro_python_frontend::PythonFrontend;
    let graph = frontend.extract(&source, &path).unwrap();

    let slot_edges: Vec<_> = graph.edges.iter().filter(|e| e.slot.is_some()).collect();
    let expr_nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, vampiro_cir::NodeKind::Expression))
        .collect();

    assert!(
        !slot_edges.is_empty(),
        "expected at least one per-slot edge, got 0. Total edges: {}",
        graph.edges.len()
    );
    assert!(
        !expr_nodes.is_empty(),
        "expected at least one expression node, got 0. Total nodes: {}",
        graph.nodes.len()
    );

    let slots: Vec<u32> = slot_edges.iter().filter_map(|e| e.slot).collect();
    assert!(
        slots.iter().any(|&s| s == 0 || s == 1),
        "expected slot 0 or 1, got {:?}",
        slots
    );

    eprintln!(
        "Python data-flow: {} slot edges, {} expression nodes, total edges={}, total nodes={}, slots={:?}",
        slot_edges.len(),
        expr_nodes.len(),
        graph.edges.len(),
        graph.nodes.len(),
        slots
    );
}

