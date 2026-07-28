/// Round-trip tests for vampiro-cir fixture files.
///
/// These tests load JSON fixture files from
/// `tests/fixtures/add-cir-plugin-platform/1/`, deserialize them into
/// `CirGraph`, and verify round-trip fidelity through JSON serialization.
use std::path::Path;
use vampiro_cir::CirGraph;

/// The relative path from the workspace root to the fixture directory.
const FIXTURE_DIR: &str = "tests/fixtures/add-cir-plugin-platform/1";

/// Resolve the fixture directory relative to the crate root.
fn fixture_path(name: &str) -> String {
    // When running tests, CARGO_MANIFEST_DIR points to the crate root.
    let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(FIXTURE_DIR)
        .join(name);
    path.to_string_lossy().to_string()
}

#[test]
fn fixture_simple_call_round_trip() {
    let path = fixture_path("simple-call.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    let graph: CirGraph = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {path}: {e}"));

    assert_eq!(graph.version, "0.1.0");
    assert_eq!(graph.source_file, "src/lib.rs");
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);

    // Round-trip: serialize back and verify equality
    let serialized =
        serde_json::to_string_pretty(&graph).unwrap_or_else(|e| panic!("failed to serialize: {e}"));
    let re_parsed: CirGraph =
        serde_json::from_str(&serialized).unwrap_or_else(|e| panic!("failed to re-parse: {e}"));
    assert_eq!(graph, re_parsed);
}

#[test]
fn fixture_recursive_effect_round_trip() {
    let path = fixture_path("recursive-effect.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    let graph: CirGraph = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {path}: {e}"));

    assert_eq!(graph.version, "0.1.0");
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.edges.len(), 1);

    // Verify recursive effect structure
    let serialized = serde_json::to_string_pretty(&graph).unwrap();
    let re_parsed: CirGraph = serde_json::from_str(&serialized).unwrap();
    assert_eq!(graph, re_parsed);
}

#[test]
fn fixture_custom_effect_round_trip() {
    let path = fixture_path("custom-effect.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    let graph: CirGraph = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {path}: {e}"));

    assert_eq!(graph.version, "0.1.0");
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.edges.len(), 1);

    // Verify round-trip
    let serialized = serde_json::to_string_pretty(&graph).unwrap();
    let re_parsed: CirGraph = serde_json::from_str(&serialized).unwrap();
    assert_eq!(graph, re_parsed);
}

#[test]
fn fixture_canonical_utf8_byte_reproducibility() {
    // Verify that deserializing and re-serializing produces the same bytes
    // (modulo key ordering which serde_json preserves)
    let path = fixture_path("simple-call.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    let graph: CirGraph = serde_json::from_str(&content).unwrap();
    let serialized = serde_json::to_string(&graph).unwrap();
    let re_parsed: CirGraph = serde_json::from_str(&serialized).unwrap();
    let re_serialized = serde_json::to_string(&re_parsed).unwrap();

    // serde_json with default settings preserves field order
    assert_eq!(
        serialized, re_serialized,
        "canonical UTF-8 serialization must be reproducible"
    );
}
