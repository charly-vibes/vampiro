//! Julia law/lifecycle/core integration tests (0vb.8.11).
//!
//! Fixtures: tests/fixtures/add-python-clojure-julia-frontends/7/

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
        .join("tests/fixtures/add-python-clojure-julia-frontends/7")
        .join(name)
}

fn read_fixture(name: &str) -> String {
    let path = fixture_path(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
}

#[test]
fn julia_runner_input_has_tagged_fns() {
    let source = read_fixture("runner-input.jl");
    let frontend = vampiro_julia_frontend::JuliaFrontend;
    let path = fixture_path("runner-input.jl");

    let graph = frontend.extract(&source, &path).unwrap();
    // 4 functions: add, greet, process, count_up
    assert_eq!(graph.nodes.len(), 4);

    let names: Vec<&str> = graph
        .nodes
        .iter()
        .filter_map(|n| n.name.as_deref())
        .collect();
    assert!(names.contains(&"add"));
    assert!(names.contains(&"greet"));
    assert!(names.contains(&"process"));
    assert!(names.contains(&"count_up"));
}

#[test]
fn julia_runner_input_via_extract_full() {
    let source = read_fixture("runner-input.jl");
    let path = fixture_path("runner-input.jl");
    let frontend = vampiro_julia_frontend::JuliaFrontend;

    let out = frontend.extract_full(&source, &path).unwrap();
    assert_eq!(out.law_input.version, "0.1.0");
    assert_eq!(out.law_input.tagged_fns.len(), 4);

    let names: Vec<&str> = out
        .law_input
        .tagged_fns
        .iter()
        .map(|f| f.name.as_str())
        .collect();
    assert!(names.contains(&"add"));
    assert!(names.contains(&"greet"));
    assert!(names.contains(&"process"));
    assert!(names.contains(&"count_up"));
}

#[test]
fn julia_lifecycle_facts_via_extract_full() {
    let source = read_fixture("lifecycle-facts.jl");
    let path = fixture_path("lifecycle-facts.jl");
    let frontend = vampiro_julia_frontend::JuliaFrontend;

    let out = frontend.extract_full(&source, &path).unwrap();
    let facts = &out.lifecycle_facts;

    assert_eq!(facts.version, "0.1.0");
    // for loop → retries, assignment → writes, return → exit paths
    assert!(
        !facts.retries.is_empty(),
        "expected retry facts from for loop"
    );
    assert!(
        !facts.writes.is_empty(),
        "expected write facts from assignments"
    );
    assert!(
        !facts.exit_paths.is_empty(),
        "expected exit paths from returns"
    );
}

#[test]
fn julia_fixtures_exist() {
    for name in &[
        "runner-input.jl",
        "lifecycle-facts.jl",
        "facade-metadata.jl",
    ] {
        let path = fixture_path(name);
        assert!(path.exists(), "missing fixture: {name} at {path:?}");
    }
}

#[test]
fn julia_fixture_source_is_parseable() {
    let frontend = vampiro_julia_frontend::JuliaFrontend;
    for name in &[
        "runner-input.jl",
        "lifecycle-facts.jl",
        "facade-metadata.jl",
    ] {
        let source = read_fixture(name);
        let path = fixture_path(name);
        let result = frontend.extract(&source, &path);
        assert!(
            result.is_ok(),
            "failed to parse {name}: {}",
            result.unwrap_err()
        );
    }
}
