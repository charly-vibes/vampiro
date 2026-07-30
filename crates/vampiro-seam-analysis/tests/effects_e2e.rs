//! E2E test for the effect-handling tracer over a hand-constructed CIR graph.
//!
//! Constructs a CIR graph with swallowed-effect edges (REQ-9, REQ-C4) and runs
//! the full seam-analysis pipeline to verify spec-conformant output.
//!
//! Note: the Rust frontend does not yet classify edges as `Swallowed`
//! (discard detection is a separate enhancement). This E2E test constructs the
//! graph programmatically to validate the analyzer + evidence + output format.

use vampiro_cir::{NodeKind, ScalarKind, 
    CirEdge, CirGraph, CirNode, DiscardSpan, EffectChannel, EffectResolution, Provenance,
    SourceSpan, StableId, Totality, UnwrapEvidence, UnwrapKind,
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

fn node(id: &str, effect: EffectChannel, line: usize) -> CirNode {
    CirNode {
        id: sid(id),
        domain: vampiro_cir::Shape::Scalar(ScalarKind::Unit),
        codomain: vampiro_cir::Shape::Scalar(ScalarKind::Unit),
        effect,
        span: span(line),
        name: Some(id.into()),
        trust_provenance: Default::default(),
        is_test: false,
        kind: NodeKind::Declaration,
        containing_function: None,
    }
}

fn edge(
    id: &str,
    source: &str,
    target: &str,
    resolution: EffectResolution,
    unwrap_evidence: Option<UnwrapEvidence>,
    discard_spans: Vec<DiscardSpan>,
    line: usize,
) -> CirEdge {
    CirEdge {
        id: sid(id),
        source: sid(source),
        target: sid(target),
        resolution,
        unwrap_evidence,
        provenance: Provenance::Direct,
        span: span(line),
        discard_spans,
        trust_provenance: Default::default(),
        slot: None,
        arg_shape: None,
    }
}

fn discard_span(line: usize) -> DiscardSpan {
    DiscardSpan {
        file: "src/lib.rs".into(),
        start_line: line,
        end_line: line,
    }
}

#[test]
fn effects_e2e_swallowed_result() {
    let mut graph = CirGraph::new("src/lib.rs");
    graph.add_node(node("total", EffectChannel::Plain, 1));
    graph.add_node(node("lookup_price", EffectChannel::Result, 5));
    graph.add_edge(edge(
        "e1",
        "total",
        "lookup_price",
        EffectResolution::Swallowed,
        None,
        vec![discard_span(3)],
        3,
    ));

    let findings = analyze(&graph);

    let robustness: Vec<_> = findings
        .iter()
        .filter(|f| f.axis == Axis::Robustness)
        .collect();
    assert_eq!(
        robustness.len(),
        1,
        "expected exactly one robustness finding; got {findings:?}"
    );

    let f = &robustness[0];

    // Required finding fields (REQ-4, REQ-9).
    assert_eq!(f.rule, "REQ-9");
    assert_eq!(f.axis, Axis::Robustness);
    assert_eq!(f.severity, Severity::Medium);
    assert_eq!(f.line_range.start, 3, "line range should match edge span");
    assert!(f.line_range.end >= f.line_range.start);
    assert_eq!(f.classification, "swallowed-effect");

    // Evidence carries the discarded channel and exact discard spans (REQ-9).
    #[allow(irrefutable_let_patterns)]
    let Evidence::SwallowedEffect {
        discarded_channel,
        discard_lines,
        totality: _,
        ancestor_handled,
    } = &f.evidence
    else {
        panic!("expected swallowed effect evidence, got {:?}", f.evidence);
    };
    assert_eq!(*discarded_channel, EffectChannel::Result);
    assert_eq!(discard_lines.len(), 1);
    assert_eq!(discard_lines[0].start_line, 3);
    assert!(
        ancestor_handled.is_none(),
        "result swallow should not trigger ancestor search"
    );
}

#[test]
fn effects_e2e_swallowed_throws_with_ancestor() {
    // bottom -> middle -> top
    // middle swallows bottom's throws; top unwraps it → ancestor handled
    let mut graph = CirGraph::new("src/lib.rs");
    graph.add_node(node("bottom", EffectChannel::Throws, 10));
    graph.add_node(node("middle", EffectChannel::Plain, 5));
    graph.add_node(node("top", EffectChannel::Plain, 1));
    graph.add_edge(edge(
        "e_mid",
        "middle",
        "bottom",
        EffectResolution::Swallowed,
        None,
        vec![discard_span(6)],
        6,
    ));
    graph.add_edge(edge(
        "e_top",
        "top",
        "middle",
        EffectResolution::Unwrapped,
        Some(UnwrapEvidence {
            kind: UnwrapKind::Ordinary,
            totality: Totality::Total,
        }),
        vec![],
        2,
    ));

    let findings = analyze(&graph);
    // Top unwraps → ancestor handles → no finding for middle's swallow.
    let robustness: Vec<_> = findings
        .iter()
        .filter(|f| f.axis == Axis::Robustness)
        .collect();
    assert!(
        robustness.is_empty(),
        "throws swallowed with ancestor handler → no finding"
    );
}

#[test]
fn effects_e2e_swallowed_throws_no_ancestor() {
    // bottom -> middle -> top
    // middle swallows bottom's throws; top propagates only → ancestor does NOT handle
    let mut graph = CirGraph::new("src/lib.rs");
    graph.add_node(node("bottom", EffectChannel::Throws, 10));
    graph.add_node(node("middle", EffectChannel::Plain, 5));
    graph.add_node(node("top", EffectChannel::Plain, 1));
    graph.add_edge(edge(
        "e_mid",
        "middle",
        "bottom",
        EffectResolution::Swallowed,
        None,
        vec![discard_span(6)],
        6,
    ));
    graph.add_edge(edge(
        "e_top",
        "top",
        "middle",
        EffectResolution::Propagated,
        None,
        vec![],
        2,
    ));

    let findings = analyze(&graph);
    let robustness: Vec<_> = findings
        .iter()
        .filter(|f| f.axis == Axis::Robustness)
        .collect();
    assert_eq!(robustness.len(), 1, "expected one robustness finding");
    #[allow(irrefutable_let_patterns)]
    let Evidence::SwallowedEffect {
        ancestor_handled, ..
    } = &robustness[0].evidence
    else {
        panic!("expected swallowed effect evidence");
    };
    assert_eq!(*ancestor_handled, Some(false));
}

#[test]
fn effects_e2e_force_partial_unwrap() {
    let mut graph = CirGraph::new("src/lib.rs");
    graph.add_node(node("caller", EffectChannel::Plain, 1));
    graph.add_node(node("callee", EffectChannel::Option, 5));
    graph.add_edge(edge(
        "e1",
        "caller",
        "callee",
        EffectResolution::Unwrapped,
        Some(UnwrapEvidence {
            kind: UnwrapKind::Force,
            totality: Totality::Partial,
        }),
        vec![discard_span(3)],
        3,
    ));

    let findings = analyze(&graph);
    let robustness: Vec<_> = findings
        .iter()
        .filter(|f| f.axis == Axis::Robustness)
        .collect();
    assert!(
        robustness.is_empty(),
        "force+partial unwrap is a panic risk, not a swallowed effect"
    );
}
