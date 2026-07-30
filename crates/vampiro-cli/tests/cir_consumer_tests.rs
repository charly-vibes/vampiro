/// Consumer compatibility test: verifies that `vampiro-cir` integrates
/// with the CLI finding and configuration contracts.
///
/// This test demonstrates that a consumer of the CIR plugin platform can:
/// 1. Import and construct CIR types
/// 2. Validate graphs (depth limits, edge references)
/// 3. Produce findings using the CLI finding contract
/// 4. Load configuration using the CLI config contract
use std::path::PathBuf;
use vampiro_cir::{NodeKind, ScalarKind, 
    CirEdge, CirError, CirGraph, CirNode, EffectChannel, EffectResolution, Frontend, Provenance,
    Shape, SourceSpan,
};
use vampiro_cli::config::Config;
use vampiro_cli::finding::{Axis, Finding, Severity};

/// A simple test frontend for consumer compatibility.
struct ConsumerTestFrontend;

impl Frontend for ConsumerTestFrontend {
    fn language(&self) -> &'static str {
        "consumer-test"
    }

    fn extract(&self, source: &str, _path: &std::path::Path) -> Result<CirGraph, CirError> {
        let mut graph = CirGraph::new(source);
        let node = CirNode {
            id: "fn-main".into(),
            domain: Shape::Scalar(ScalarKind::Unit),
            codomain: Shape::Scalar(ScalarKind::Unit),
            effect: EffectChannel::Plain,
            span: SourceSpan {
                file: source.into(),
                start_line: 1,
                start_column: 1,
                end_line: 3,
                end_column: 1,
            },
            name: Some("main".into()),
            trust_provenance: Default::default(),
            is_test: false,
            kind: NodeKind::Declaration,
            containing_function: None,
        };
        graph.add_node(node);
        Ok(graph)
    }
}

#[test]
fn consumer_imports_cir_and_constructs_graph() {
    // A consumer should be able to import and construct CirGraph, CirNode, CirEdge.
    let mut graph = CirGraph::new("src/lib.rs");

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
        name: Some("caller".into()),
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
        name: Some("callee".into()),
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

    assert_eq!(graph.version, "0.3.0");
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
}

#[test]
fn consumer_validates_graph() {
    // A consumer should be able to call validate() on a constructed graph.
    let mut graph = CirGraph::new("test.rs");

    let node = CirNode {
        id: "n1".into(),
        domain: Shape::Scalar(ScalarKind::Unit),
        codomain: Shape::Scalar(ScalarKind::Unit),
        effect: EffectChannel::Plain,
        span: SourceSpan {
            file: "test.rs".into(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
        },
        name: None,
        trust_provenance: Default::default(),
        is_test: false,
        kind: NodeKind::Declaration,
        containing_function: None,
    };
    graph.add_node(node);

    let result = graph.validate();
    assert!(result.is_ok(), "valid graph should pass validation");
}

#[test]
fn consumer_detects_invalid_graph() {
    // A consumer should be able to detect invalid graphs.
    let mut graph = CirGraph::new("test.rs");

    let node = CirNode {
        id: "n1".into(),
        domain: Shape::Scalar(ScalarKind::Unit),
        codomain: Shape::Scalar(ScalarKind::Unit),
        effect: EffectChannel::Plain,
        span: SourceSpan {
            file: "test.rs".into(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
        },
        name: None,
        trust_provenance: Default::default(),
        is_test: false,
        kind: NodeKind::Declaration,
        containing_function: None,
    };
    let edge = CirEdge {
        id: "e1".into(),
        source: "n1".into(),
        target: "n2".into(), // n2 doesn't exist
        resolution: EffectResolution::Propagated,
        unwrap_evidence: None,
        provenance: Provenance::Direct,
        span: SourceSpan {
            file: "test.rs".into(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
        },
        discard_spans: vec![],
        trust_provenance: Default::default(),
        slot: None,
        arg_shape: None,
    };
    graph.add_node(node);
    graph.add_edge(edge);

    let result = graph.validate();
    assert!(
        result.is_err(),
        "graph with missing target should fail validation"
    );
    match result.unwrap_err() {
        CirError::MissingNode { .. } => {} // expected
        other => panic!("expected MissingNode, got: {other:?}"),
    }
}

#[test]
fn consumer_uses_frontend_trait() {
    // A consumer should be able to implement the Frontend trait.
    let frontend = ConsumerTestFrontend;
    assert_eq!(frontend.language(), "consumer-test");

    let graph = frontend
        .extract("src/main.rs", std::path::Path::new("src/main.rs"))
        .expect("consumer frontend should extract successfully");
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.edges.len(), 0);

    // Extracted graph must pass validation
    let result = graph.validate();
    assert!(
        result.is_ok(),
        "consumer frontend graph must pass validation"
    );
}

#[test]
fn consumer_uses_cli_finding_contract() {
    // A consumer should be able to use the CLI finding contract alongside CIR.
    let finding = Finding::composition_mismatch(
        PathBuf::from("src/lib.rs"),
        10..=15,
        Shape::Scalar(ScalarKind::Unit),
        Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]),
        vec![Shape::Opaque],
    );

    assert_eq!(finding.rule, "REQ-7");
    assert_eq!(finding.severity, Severity::Medium);
    assert_eq!(finding.axis, Axis::Composition);
    assert_eq!(finding.classification, "composition-break");

    // Serialize and deserialize
    let json = serde_json::to_string(&finding).unwrap();
    let deserialized: Finding = serde_json::from_str(&json).unwrap();
    assert_eq!(finding, deserialized);
}

#[test]
fn consumer_uses_cli_config_contract() {
    // A consumer should be able to construct a default configuration.
    let config = Config::default();
    // Default config has no scan_threads configured
    assert_eq!(config.scan_threads, None);
}

#[test]
fn consumer_round_trips_cir_via_json() {
    // A consumer should be able to serialize CIR graphs to JSON and back.
    let mut graph = CirGraph::new("consumer_test.rs");

    let node = CirNode {
        id: "n1".into(),
        domain: Shape::Scalar(ScalarKind::Unit),
        codomain: Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit)]),
        effect: EffectChannel::Option,
        span: SourceSpan {
            file: "consumer_test.rs".into(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 1,
        },
        name: Some("consumer_fn".into()),
        trust_provenance: Default::default(),
        is_test: false,
        kind: NodeKind::Declaration,
        containing_function: None,
    };
    graph.add_node(node);

    let json = serde_json::to_string_pretty(&graph).unwrap();
    let deserialized: CirGraph = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.nodes.len(), 1);
    assert_eq!(deserialized.nodes[0].name.as_deref(), Some("consumer_fn"));
    assert_eq!(deserialized.nodes[0].effect, EffectChannel::Option);
    assert_eq!(
        deserialized.nodes[0].codomain,
        Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit)])
    );
}

#[test]
fn consumer_from_json_validates() {
    // A consumer should be able to use from_json which validates on deserialization.
    let json = r#"{
        "version": "0.3.0",
        "source_file": "test.rs",
        "nodes": [
            {
                "id": "n1",
                "domain": {"scalar": "unit"},
                "codomain": {"scalar": "unit"},
                "effect": "plain",
                "span": {
                    "file": "test.rs",
                    "start_line": 1,
                    "start_column": 1,
                    "end_line": 1,
                    "end_column": 1
                }
            }
        ],
        "edges": []
    }"#;

    let graph = CirGraph::from_json(json).unwrap();
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.edges.len(), 0);
}

#[test]
fn consumer_rejects_invalid_json() {
    // A consumer should be able to detect invalid JSON.
    let json = r#"{
        "version": "0.2.1",
        "source_file": "test.rs",
        "nodes": [],
        "edges": [
            {
                "id": "e1",
                "source": "n1",
                "target": "n2",
                "resolution": "propagated",
                "provenance": "direct",
                "span": {
                    "file": "test.rs",
                    "start_line": 1,
                    "start_column": 1,
                    "end_line": 1,
                    "end_column": 1
                },
                "discard_spans": []
            }
        ]
    }"#;

    let result = CirGraph::from_json(json);
    assert!(result.is_err(), "from_json must reject invalid graph");
    match result.unwrap_err() {
        CirError::MissingNode { .. } => {} // expected
        other => panic!("expected MissingNode, got: {other:?}"),
    }
}
