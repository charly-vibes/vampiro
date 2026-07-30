/// Fixture-based conformance tests for the Frontend trait.
///
/// These tests load JSON fixture files from
/// `tests/fixtures/add-cir-plugin-platform/3/`, verify the Frontend trait
/// contract (language identifier, graph extraction, depth-limit rejection),
/// and test byte reproducibility.
use serde::Deserialize;
use std::path::Path;
use vampiro_cir::{NodeKind, ScalarKind, 
    CirEdge, CirError, CirGraph, CirNode, EffectChannel, EffectResolution, Frontend, Provenance,
    Shape, SourceSpan,
};

/// The relative path from the workspace root to the fixture directory.
const FIXTURE_DIR: &str = "tests/fixtures/add-cir-plugin-platform/3";

/// Resolve the fixture directory relative to the crate root.
fn fixture_path(name: &str) -> String {
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

/// Deserialize a CirGraph from JSON, disabling serde_json's recursion limit
/// so that deeply nested fixtures (e.g., depth-exceeded shapes) can be loaded.
fn deserialize_graph(json: &str) -> Result<CirGraph, CirError> {
    let mut de = serde_json::Deserializer::from_str(json);
    de.disable_recursion_limit();
    let graph =
        CirGraph::deserialize(&mut de).map_err(|e| CirError::Deserialization(e.to_string()))?;
    Ok(graph)
}

// --- Mock frontends for conformance testing ---

/// A frontend that produces a known valid CirGraph matching the
/// `valid-extraction.json` fixture.
struct MockValidFrontend;

impl Frontend for MockValidFrontend {
    fn language(&self) -> &'static str {
        "mock-valid"
    }

    fn extract(&self, source: &str, _path: &Path) -> Result<CirGraph, CirError> {
        // Produce a graph with the same structure as the fixture
        let mut graph = CirGraph::new(source);
        let caller = CirNode {
            id: "caller".into(),
            domain: Shape::Scalar(ScalarKind::Unit),
            codomain: Shape::Scalar(ScalarKind::Unit),
            effect: EffectChannel::Plain,
            span: SourceSpan {
                file: "src/lib.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 3,
                end_column: 1,
            },
            name: Some("caller_fn".into()),
            trust_provenance: Default::default(),
            is_test: false,
            kind: NodeKind::Declaration,
            containing_function: None,
        };
        let callee = CirNode {
            id: "callee".into(),
            domain: Shape::Scalar(ScalarKind::Unit),
            codomain: Shape::Scalar(ScalarKind::Unit),
            effect: EffectChannel::Plain,
            span: SourceSpan {
                file: "src/lib.rs".into(),
                start_line: 5,
                start_column: 1,
                end_line: 7,
                end_column: 1,
            },
            name: Some("callee_fn".into()),
            trust_provenance: Default::default(),
            is_test: false,
            kind: NodeKind::Declaration,
            containing_function: None,
        };
        let edge = CirEdge {
            id: "call-1".into(),
            source: "caller".into(),
            target: "callee".into(),
            resolution: EffectResolution::Propagated,
            unwrap_evidence: None,
            provenance: Provenance::Direct,
            span: SourceSpan {
                file: "src/lib.rs".into(),
                start_line: 2,
                start_column: 5,
                end_line: 2,
                end_column: 15,
            },
            discard_spans: vec![],
            trust_provenance: Default::default(),
            slot: None,
            arg_shape: None,
        };
        graph.add_node(caller);
        graph.add_node(callee);
        graph.add_edge(edge);
        Ok(graph)
    }
}

/// A frontend that produces a graph with an effect depth exceeding the limit.
struct MockDepthExceededEffectFrontend;

impl Frontend for MockDepthExceededEffectFrontend {
    fn language(&self) -> &'static str {
        "mock-depth-effect"
    }

    fn extract(&self, _source: &str, _path: &Path) -> Result<CirGraph, CirError> {
        let mut graph = CirGraph::new("deep.rs");
        let mut deep_effect = EffectChannel::Plain;
        for _ in 0..(vampiro_cir::effect::MAX_EFFECT_DEPTH + 1) {
            deep_effect = EffectChannel::Recursive(Box::new(deep_effect));
        }
        let node = CirNode {
            id: "deep-node".into(),
            domain: Shape::Scalar(ScalarKind::Unit),
            codomain: Shape::Scalar(ScalarKind::Unit),
            effect: deep_effect,
            span: SourceSpan {
                file: "deep.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
            },
            name: Some("deep_fn".into()),
            trust_provenance: Default::default(),
            is_test: false,
            kind: NodeKind::Declaration,
            containing_function: None,
        };
        graph.add_node(node);
        // The frontend produces the graph, but it should be rejected by validate()
        // or returned as an error. Here we return it and let the consumer validate.
        Ok(graph)
    }
}

/// A frontend that produces a graph with a shape depth exceeding the limit.
struct MockDepthExceededShapeFrontend;

impl Frontend for MockDepthExceededShapeFrontend {
    fn language(&self) -> &'static str {
        "mock-depth-shape"
    }

    fn extract(&self, _source: &str, _path: &Path) -> Result<CirGraph, CirError> {
        let mut graph = CirGraph::new("deep.rs");
        let mut deep_shape = Shape::Scalar(ScalarKind::Unit);
        for _ in 0..(vampiro_cir::shape::MAX_SHAPE_DEPTH + 1) {
            deep_shape = Shape::Record(vec![deep_shape]);
        }
        let node = CirNode {
            id: "deep-node".into(),
            domain: deep_shape,
            codomain: Shape::Scalar(ScalarKind::Unit),
            effect: EffectChannel::Plain,
            span: SourceSpan {
                file: "deep.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 1,
            },
            name: Some("deep_fn".into()),
            trust_provenance: Default::default(),
            is_test: false,
            kind: NodeKind::Declaration,
            containing_function: None,
        };
        graph.add_node(node);
        Ok(graph)
    }
}

/// A frontend that always fails extraction.
struct MockFailingFrontend;

impl Frontend for MockFailingFrontend {
    fn language(&self) -> &'static str {
        "mock-fail"
    }

    fn extract(&self, _source: &str, _path: &Path) -> Result<CirGraph, CirError> {
        Err(CirError::Extraction(
            "syntax error: unexpected token at line 1".into(),
        ))
    }
}

// --- Conformance tests: Frontend trait contract ---

#[test]
fn frontend_has_language_identifier() {
    let frontend = MockValidFrontend;
    assert_eq!(frontend.language(), "mock-valid");

    let failing = MockFailingFrontend;
    assert_eq!(failing.language(), "mock-fail");

    let depth_effect = MockDepthExceededEffectFrontend;
    assert_eq!(depth_effect.language(), "mock-depth-effect");

    let depth_shape = MockDepthExceededShapeFrontend;
    assert_eq!(depth_shape.language(), "mock-depth-shape");
}

#[test]
fn frontend_language_is_static_str() {
    // The language() return type must be &'static str.
    // This compiles only if the mock frontends satisfy the contract.
    let _: &'static str = MockValidFrontend.language();
    let _: &'static str = MockFailingFrontend.language();
    let _: &'static str = MockDepthExceededEffectFrontend.language();
    let _: &'static str = MockDepthExceededShapeFrontend.language();
}

#[test]
fn frontend_valid_extraction_produces_graph() {
    let frontend = MockValidFrontend;
    let graph = frontend
        .extract("src/lib.rs", Path::new("src/lib.rs"))
        .expect("valid extraction should succeed");
    assert_eq!(graph.version, "0.3.0");
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
}

#[test]
fn frontend_valid_extraction_validates() {
    // After extraction, the graph must pass validation.
    let frontend = MockValidFrontend;
    let graph = frontend
        .extract("src/lib.rs", Path::new("src/lib.rs"))
        .unwrap();
    let result = graph.validate();
    assert!(
        result.is_ok(),
        "valid extraction should produce a validatable graph: {result:?}"
    );
}

#[test]
fn frontend_failing_extraction_returns_error() {
    let frontend = MockFailingFrontend;
    let result = frontend.extract("bad.rs", Path::new("bad.rs"));
    assert!(result.is_err(), "failing frontend should return an error");
    match result.unwrap_err() {
        CirError::Extraction(msg) => {
            assert!(
                msg.contains("syntax error"),
                "expected syntax error message, got: {msg}"
            );
        }
        other => panic!("expected Extraction error, got: {other:?}"),
    }
}

#[test]
fn frontend_depth_exceeded_effect_is_rejected() {
    let frontend = MockDepthExceededEffectFrontend;
    let graph = frontend
        .extract("deep.rs", Path::new("deep.rs"))
        .expect("depth-exceeded frontend should produce a graph");
    let result = graph.validate();
    assert!(
        result.is_err(),
        "graph with exceeded effect depth should fail validation"
    );
    match result.unwrap_err() {
        CirError::EffectDepthExceeded {
            max_depth,
            observed,
        } => {
            assert_eq!(max_depth, 64);
            assert!(observed > 64);
        }
        other => panic!("expected EffectDepthExceeded, got: {other:?}"),
    }
}

#[test]
fn frontend_depth_exceeded_shape_is_rejected() {
    let frontend = MockDepthExceededShapeFrontend;
    let graph = frontend
        .extract("deep.rs", Path::new("deep.rs"))
        .expect("depth-exceeded shape frontend should produce a graph");
    let result = graph.validate();
    assert!(
        result.is_err(),
        "graph with exceeded shape depth should fail validation"
    );
    match result.unwrap_err() {
        CirError::ShapeDepthExceeded {
            max_depth,
            observed,
        } => {
            assert_eq!(max_depth, 64);
            assert!(observed > 64);
        }
        other => panic!("expected ShapeDepthExceeded, got: {other:?}"),
    }
}

#[test]
fn frontend_byte_reproducibility() {
    // A conformant frontend must produce the same bytes every time
    // on unchanged input.
    let frontend = MockValidFrontend;

    let graph1 = frontend
        .extract("src/lib.rs", Path::new("src/lib.rs"))
        .unwrap();
    let graph2 = frontend
        .extract("src/lib.rs", Path::new("src/lib.rs"))
        .unwrap();

    let bytes1 = serde_json::to_vec(&graph1).unwrap();
    let bytes2 = serde_json::to_vec(&graph2).unwrap();

    assert_eq!(
        bytes1, bytes2,
        "frontend must produce byte-for-byte identical results on unchanged input"
    );
}

#[test]
fn frontend_null_contract() {
    // The NullFrontend is a built-in conformant frontend.
    let frontend = vampiro_cir::frontend::NullFrontend;
    assert_eq!(frontend.language(), "null");

    let graph = frontend.extract("", Path::new("empty.rs")).unwrap();
    assert_eq!(graph.nodes.len(), 0);
    assert_eq!(graph.edges.len(), 0);

    // NullFrontend must always produce validatable graphs.
    let result = graph.validate();
    assert!(
        result.is_ok(),
        "NullFrontend graph must pass validation: {result:?}"
    );
}

// --- Fixture conformance tests ---

#[test]
fn fixture_valid_extraction_round_trip() {
    let path = fixture_path("valid-extraction.json");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"));
    let graph: CirGraph = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to deserialize fixture {path}: {e}"));

    assert_eq!(graph.version, "0.1.0");
    assert_eq!(graph.source_file, "src/lib.rs");
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);

    // Round-trip verification
    let serialized = serde_json::to_string_pretty(&graph).unwrap();
    let re_parsed: CirGraph = serde_json::from_str(&serialized).unwrap();
    assert_eq!(graph, re_parsed, "fixture round-trip must be lossless");
}

#[test]
fn fixture_valid_extraction_passes_validation() {
    let path = fixture_path("valid-extraction.json");
    let content = std::fs::read_to_string(&path).unwrap();
    let graph: CirGraph = serde_json::from_str(&content).unwrap();
    let result = graph.validate();
    assert!(
        result.is_ok(),
        "valid extraction fixture must pass validation: {result:?}"
    );
}

#[test]
fn fixture_depth_exceeded_effect_is_rejected() {
    let path = fixture_path("depth-exceeded-effect.json");
    let content = std::fs::read_to_string(&path).unwrap();
    // Use deserialize_graph to bypass serde_json's recursion limit
    let graph = deserialize_graph(&content).unwrap();

    let result = graph.validate();
    assert!(
        result.is_err(),
        "depth-exceeded effect fixture must fail validation"
    );
    match result.unwrap_err() {
        CirError::EffectDepthExceeded { .. } => {} // expected
        other => panic!("expected EffectDepthExceeded, got: {other:?}"),
    }
}

#[test]
fn fixture_depth_exceeded_shape_is_rejected() {
    let path = fixture_path("depth-exceeded-shape.json");
    let content = std::fs::read_to_string(&path).unwrap();
    // Use deserialize_graph to bypass serde_json's recursion limit
    let graph = deserialize_graph(&content).unwrap();

    let result = graph.validate();
    assert!(
        result.is_err(),
        "depth-exceeded shape fixture must fail validation"
    );
    match result.unwrap_err() {
        CirError::ShapeDepthExceeded { .. } => {} // expected
        other => panic!("expected ShapeDepthExceeded, got: {other:?}"),
    }
}

#[test]
fn fixture_canonical_utf8_byte_reproducibility() {
    // Verify that deserializing and re-serializing produces the same bytes.
    let path = fixture_path("valid-extraction.json");
    let content = std::fs::read_to_string(&path).unwrap();
    let graph: CirGraph = serde_json::from_str(&content).unwrap();
    let serialized = serde_json::to_string(&graph).unwrap();
    let re_parsed: CirGraph = serde_json::from_str(&serialized).unwrap();
    let re_serialized = serde_json::to_string(&re_parsed).unwrap();

    assert_eq!(
        serialized, re_serialized,
        "canonical UTF-8 serialization must be reproducible"
    );
}
