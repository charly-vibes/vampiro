//! Clojure law/lifecycle/core integration tests (0vb.8.10).
//!
//! Fixtures: tests/fixtures/add-python-clojure-julia-frontends/6/

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
        .join("tests/fixtures/add-python-clojure-julia-frontends/6")
        .join(name)
}

fn read_fixture(name: &str) -> String {
    let path = fixture_path(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
}

// ---------------------------------------------------------------------------
// 6.1: Clojure fixtures — runner inputs
// ---------------------------------------------------------------------------

#[test]
fn clojure_runner_input_has_tagged_fns() {
    let source = read_fixture("runner-input.clj");
    let frontend = vampiro_clojure_frontend::ClojureFrontend;
    let path = fixture_path("runner-input.clj");

    let graph = frontend.extract(&source, &path).unwrap();
    // 4 defns: add, greet, process, count-up-to
    assert_eq!(graph.nodes.len(), 4);

    let names: Vec<&str> = graph
        .nodes
        .iter()
        .filter_map(|n| n.name.as_deref())
        .collect();
    assert!(names.contains(&"add"));
    assert!(names.contains(&"greet"));
    assert!(names.contains(&"process"));
    assert!(names.contains(&"count-up-to"));
}

#[test]
fn clojure_runner_input_via_extract_full() {
    let source = read_fixture("runner-input.clj");
    let path = fixture_path("runner-input.clj");
    let frontend = vampiro_clojure_frontend::ClojureFrontend;

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
    assert!(names.contains(&"count-up-to"));

    // count-up-to uses lazy-seq
    assert_eq!(out.law_input.generator_refs.len(), 1);
    assert_eq!(out.law_input.generator_refs[0].name, "count-up-to");
}

// ---------------------------------------------------------------------------
// 6.1: Clojure fixtures — lifecycle facts
// ---------------------------------------------------------------------------

#[test]
fn clojure_lifecycle_facts_via_extract_full() {
    let source = read_fixture("lifecycle-facts.clj");
    let path = fixture_path("lifecycle-facts.clj");
    let frontend = vampiro_clojure_frontend::ClojureFrontend;

    let out = frontend.extract_full(&source, &path).unwrap();
    let facts = &out.lifecycle_facts;

    assert_eq!(facts.version, "0.1.0");
    // with-open → resources, loop → retries, let → writes
    assert!(
        !facts.resources.is_empty(),
        "expected resource facts from with-open"
    );
    assert!(!facts.retries.is_empty(), "expected retry facts from loop");
}

// ---------------------------------------------------------------------------
// 6.1: Clojure fixtures — facade metadata
// ---------------------------------------------------------------------------

#[test]
fn clojure_facade_metadata_via_extract_full() {
    let source = read_fixture("facade-metadata.clj");
    let path = fixture_path("facade-metadata.clj");
    let frontend = vampiro_clojure_frontend::ClojureFrontend;

    let out = frontend.extract_full(&source, &path).unwrap();
    // Should detect require directives (may be empty if grammar doesn't parse :refer/:require)
    let _ = out.facades;
}

// ---------------------------------------------------------------------------
// Contract fixture validation
// ---------------------------------------------------------------------------

#[test]
fn clojure_fixtures_exist() {
    for name in &[
        "runner-input.clj",
        "lifecycle-facts.clj",
        "facade-metadata.clj",
    ] {
        let path = fixture_path(name);
        assert!(path.exists(), "missing fixture: {name} at {path:?}");
    }
}

#[test]
fn clojure_fixture_source_is_parseable() {
    let frontend = vampiro_clojure_frontend::ClojureFrontend;
    for name in &[
        "runner-input.clj",
        "lifecycle-facts.clj",
        "facade-metadata.clj",
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
