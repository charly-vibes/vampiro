//! E2E test for the redundancy tracer over a hand-constructed CIR graph.
//!
//! Constructs a CIR graph with multiple branches feeding a consumer node with
//! mismatched codomain shapes (REQ-11, REQ-C7).

use vampiro_cir::{NodeKind, ScalarKind, 
    CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, Provenance, Shape, SourceSpan,
    StableId,
};
use vampiro_seam_analysis::{analyze, Axis, Evidence, Severity};

fn sid(s: &str) -> StableId {
    StableId::new(s)
}

fn span(line: usize) -> SourceSpan {
    SourceSpan {
        file: "src/lib.rs".into(),
        start_line: line,
        start_column: 1,
        end_line: line,
        end_column: 10,
    }
}

fn node(id: &str, domain: Shape, codomain: Shape, line: usize) -> CirNode {
    CirNode {
        id: sid(id),
        domain,
        codomain,
        effect: EffectChannel::Plain,
        span: span(line),
        name: Some(id.into()),
        trust_provenance: Default::default(),
        is_test: false,
        kind: NodeKind::Declaration,
        containing_function: None,
    }
}

fn edge(id: &str, source: &str, target: &str, line: usize) -> CirEdge {
    CirEdge {
        id: sid(id),
        source: sid(source),
        target: sid(target),
        resolution: EffectResolution::Propagated,
        unwrap_evidence: None,
        provenance: Provenance::Direct,
        span: span(line),
        discard_spans: Vec::new(),
        trust_provenance: Default::default(),
        slot: None,
        arg_shape: None,
    }
}

#[test]
fn redundancy_e2e_two_branches_mismatch() {
    let mut graph = CirGraph::new("src/lib.rs");
    // primary -> FullRecord (Record[Scalar, Scalar])
    graph.add_node(node(
        "primary_source_fetch",
        Shape::Scalar(ScalarKind::Unit),
        Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit)]),
        1,
    ));
    // cache -> Option<f64> (Parameterized{Option, [Scalar]})
    graph.add_node(node(
        "cache_get",
        Shape::Scalar(ScalarKind::Unit),
        Shape::Parameterized {
            base: "Option".into(),
            parameters: vec![Shape::Scalar(ScalarKind::Unit)],
        },
        5,
    ));
    // use -> expects (f64, String) = Record[Scalar, Scalar]
    graph.add_node(node(
        "use_data",
        Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit)]),
        Shape::Scalar(ScalarKind::Unit),
        10,
    ));
    graph.add_edge(edge("e1", "primary_source_fetch", "use_data", 3));
    graph.add_edge(edge("e2", "cache_get", "use_data", 7));

    let findings = analyze(&graph);
    let redundancy: Vec<_> = findings
        .iter()
        .filter(|f| f.axis == Axis::Robustness && f.classification == "redundancy-mismatch")
        .collect();
    assert_eq!(
        redundancy.len(),
        1,
        "expected exactly one redundancy finding"
    );

    let f = &redundancy[0];
    assert_eq!(f.rule, "REQ-11");
    assert_eq!(f.axis, Axis::Robustness);
    assert_eq!(f.severity, Severity::Medium);
    assert_eq!(f.classification, "redundancy-mismatch");

    #[allow(irrefutable_let_patterns)]
    let Evidence::RedundancyMismatch {
        branch_shapes,
        expected_shape,
        adapters,
    } = &f.evidence
    else {
        panic!("expected redundancy mismatch evidence");
    };
    assert_eq!(branch_shapes.len(), 2);
    assert_eq!(
        *expected_shape,
        Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit)])
    );
    assert!(adapters.is_empty(), "no adapters should be found");
}

#[test]
fn redundancy_e2e_three_branches() {
    let mut graph = CirGraph::new("src/lib.rs");
    graph.add_node(node("a", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 1));
    graph.add_node(node(
        "b",
        Shape::Scalar(ScalarKind::Unit),
        Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit)]),
        5,
    ));
    graph.add_node(node(
        "c",
        Shape::Scalar(ScalarKind::Unit),
        Shape::Parameterized {
            base: "Option".into(),
            parameters: vec![Shape::Scalar(ScalarKind::Unit)],
        },
        9,
    ));
    graph.add_node(node("use", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 15));
    graph.add_edge(edge("e1", "a", "use", 3));
    graph.add_edge(edge("e2", "b", "use", 7));
    graph.add_edge(edge("e3", "c", "use", 11));

    let findings = analyze(&graph);
    let redundancy: Vec<_> = findings
        .iter()
        .filter(|f| f.classification == "redundancy-mismatch")
        .collect();
    assert_eq!(
        redundancy.len(),
        1,
        "3 branches with different shapes → finding"
    );
    #[allow(irrefutable_let_patterns)]
    let Evidence::RedundancyMismatch { branch_shapes, .. } = &redundancy[0].evidence
    else {
        panic!("expected redundancy mismatch");
    };
    assert_eq!(branch_shapes.len(), 3);
}

#[test]
fn redundancy_e2e_all_same_no_finding() {
    let mut graph = CirGraph::new("src/lib.rs");
    graph.add_node(node("a", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 1));
    graph.add_node(node("b", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 5));
    graph.add_node(node("use", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 10));
    graph.add_edge(edge("e1", "a", "use", 3));
    graph.add_edge(edge("e2", "b", "use", 7));

    let findings = analyze(&graph);
    let redundancy: Vec<_> = findings
        .iter()
        .filter(|f| f.classification == "redundancy-mismatch")
        .collect();
    assert!(
        redundancy.is_empty(),
        "same codomain → no redundancy finding"
    );
}
