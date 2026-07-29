//! Boundary-leak analysis: flags untrusted data flowing into interior nodes
//! that are not trust-boundary sources.
//!
//! A boundary leak occurs when:
//! 1. An edge carries `trust_provenance = Untrusted`
//! 2. The target node is NOT a trust-boundary source
//!
//! Trust-boundary sources are nodes that produce untrusted data without
//! consuming untrusted input (the data enters the system at that point).
//! Smart constructors are NOT recognized in the general case — they require
//! explicit project configuration or conformance-idiom matching (future
//! addition). Without configuration, any edge carrying untrusted data into
//! an interior node IS a boundary leak.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::finding::{Axis, Evidence, Finding, Severity};
use vampiro_cir::{CirGraph, TrustProvenance};

/// Analyzer for boundary-leak findings (REQ-B3, REQ-C4).
#[derive(Debug, Default)]
pub struct BoundaryLeakAnalyzer;

impl BoundaryLeakAnalyzer {
    /// Create a new boundary-leak analyzer.
    pub fn new() -> Self {
        BoundaryLeakAnalyzer
    }

    /// Analyze the graph for boundary-leak findings.
    ///
    /// Checks every edge with `trust_provenance = Untrusted` flowing into a
    /// target node that is not a trust-boundary source. Emits exactly one
    /// `robustness` finding at `HIGH` severity per violating edge.
    ///
    /// Trust-boundary sources are nodes that produce untrusted data without
    /// consuming untrusted input (the data enters the system at that point).
    /// Smart constructors are NOT recognized in the general case — they
    /// require explicit project configuration or conformance-idom matching
    /// (to be added in a later pass). Without that, any edge carrying
    /// untrusted data into an interior node IS a boundary leak.
    pub fn analyze(&self, graph: &CirGraph) -> Vec<Finding> {
        let boundary_sources = self.identify_boundary_sources(graph);

        let mut findings = Vec::new();

        for edge in &graph.edges {
            if edge.trust_provenance != TrustProvenance::Untrusted {
                continue;
            }

            let target = match graph.node_by_id(&edge.target) {
                Some(n) => n,
                None => continue,
            };

            // Skip if target is a boundary source (where untrusted enters)
            if boundary_sources.contains(&edge.target) {
                continue;
            }

            // Found a boundary leak
            let source_name = graph
                .node_by_id(&edge.source)
                .and_then(|n| n.name.clone())
                .unwrap_or_else(|| edge.source.to_string());
            let target_name = target
                .name
                .clone()
                .unwrap_or_else(|| edge.target.to_string());

            let finding = Finding {
                rule: "REQ-B3".into(),
                path: PathBuf::from(&edge.span.file),
                line_range: (edge.span.start_line..=edge.span.end_line).into(),
                severity: Severity::High,
                axis: Axis::Robustness,
                filtration_distance: None,
                evidence: Evidence::BoundaryLeak {
                    source: edge.source.to_string(),
                    source_name,
                    edge_id: edge.id.to_string(),
                    target: edge.target.to_string(),
                    target_name,
                },
                classification: "boundary-leak".into(),
            };
            findings.push(finding);
        }

        findings
    }

    /// Identify trust-boundary sources: nodes whose output is `Untrusted` but
    /// that have no incoming edges carrying `Untrusted` trust provenance.
    /// This means the node generates untrusted data rather than receiving it
    /// from elsewhere in the system.
    fn identify_boundary_sources(&self, graph: &CirGraph) -> HashSet<vampiro_cir::StableId> {
        let mut sources = HashSet::new();

        // Find all incoming edges with untrusted provenance for each node
        // (excluding self-loops, which don't make a node a non-source)
        let mut untrusted_incoming: HashSet<&vampiro_cir::StableId> = HashSet::new();
        for edge in &graph.edges {
            if edge.trust_provenance == TrustProvenance::Untrusted && edge.source != edge.target {
                untrusted_incoming.insert(&edge.target);
            }
        }

        for node in &graph.nodes {
            // Node produces untrusted output
            if node.trust_provenance == TrustProvenance::Untrusted {
                // But has no untrusted incoming edges -> it generates untrusted data
                if !untrusted_incoming.contains(&node.id) {
                    sources.insert(node.id.clone());
                }
            }
        }

        sources
    }
}

#[cfg(test)]
mod tests {
    use vampiro_cir::ScalarKind;
    use super::*;
    use crate::finding::Evidence;
    use vampiro_cir::{
        CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, Provenance, Shape, SourceSpan,
        StableId,
    };

    fn sid(id: &str) -> StableId {
        StableId::new(id)
    }

    fn node(id: &str, name: &str, trust: TrustProvenance) -> CirNode {
        CirNode {
            id: sid(id),
            domain: Shape::Scalar(ScalarKind::Unit),
            codomain: Shape::Scalar(ScalarKind::Unit),
            effect: EffectChannel::Plain,
            span: SourceSpan {
                file: "lib.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 10,
            },
            name: Some(name.into()),
            trust_provenance: trust,
            is_test: false,
        }
    }

    fn edge(id: &str, source: &str, target: &str, trust: TrustProvenance, line: usize) -> CirEdge {
        CirEdge {
            id: sid(id),
            source: sid(source),
            target: sid(target),
            resolution: EffectResolution::Propagated,
            unwrap_evidence: None,
            provenance: Provenance::Direct,
            span: SourceSpan {
                file: "lib.rs".into(),
                start_line: line,
                start_column: 1,
                end_line: line,
                end_column: 20,
            },
            discard_spans: vec![],
            trust_provenance: trust,
            slot: None,
            arg_shape: None,
        }
    }

    #[test]
    fn untrusted_data_flowing_into_interior_node_emits_finding() {
        let mut graph = CirGraph::new("lib.rs");
        graph.add_node(node(
            "input_src",
            "read_request",
            TrustProvenance::Untrusted,
        ));
        graph.add_node(node("processor", "process", TrustProvenance::Trusted));
        graph.add_edge(edge(
            "e1",
            "input_src",
            "processor",
            TrustProvenance::Untrusted,
            5,
        ));

        let findings = BoundaryLeakAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 1, "expected one boundary-leak finding");

        let f = &findings[0];
        assert_eq!(f.rule, "REQ-B3");
        assert_eq!(f.severity, Severity::High);
        assert_eq!(f.axis, Axis::Robustness);
        assert_eq!(f.classification, "boundary-leak");

        match &f.evidence {
            Evidence::BoundaryLeak { source, target, .. } => {
                assert_eq!(source, &sid("input_src").to_string());
                assert_eq!(target, &sid("processor").to_string());
            }
            other => panic!("expected BoundaryLeak evidence, got {other:?}"),
        }
    }

    #[test]
    fn untrusted_flow_to_boundary_source_emits_no_finding() {
        let mut graph = CirGraph::new("lib.rs");
        graph.add_node(node("source", "read_request", TrustProvenance::Untrusted));
        graph.add_edge(edge(
            "e1",
            "source",
            "source",
            TrustProvenance::Untrusted,
            5,
        ));

        let findings = BoundaryLeakAnalyzer::new().analyze(&graph);
        assert!(
            findings.is_empty(),
            "boundary source should not trigger leak"
        );
    }

    #[test]
    fn untrusted_flow_into_smart_constructor_emits_finding() {
        // Without explicit configuration, receiving untrusted input and
        // producing trusted output is NOT proof of being a smart constructor.
        // The edge into such a node IS a boundary leak.
        let mut graph = CirGraph::new("lib.rs");
        graph.add_node(node(
            "input_src",
            "read_request",
            TrustProvenance::Untrusted,
        ));
        graph.add_node(node("validator", "validate_user", TrustProvenance::Trusted));
        graph.add_edge(edge(
            "e1",
            "input_src",
            "validator",
            TrustProvenance::Untrusted,
            5,
        ));

        let findings = BoundaryLeakAnalyzer::new().analyze(&graph);
        assert_eq!(
            findings.len(),
            1,
            "untrusted to trusted node should trigger leak without explicit config"
        );
    }

    #[test]
    fn unknown_trust_provenance_emits_no_finding() {
        let mut graph = CirGraph::new("lib.rs");
        graph.add_node(node("input_src", "read_request", TrustProvenance::Unknown));
        graph.add_node(node("processor", "process", TrustProvenance::Trusted));
        graph.add_edge(edge(
            "e1",
            "input_src",
            "processor",
            TrustProvenance::Unknown,
            5,
        ));

        let findings = BoundaryLeakAnalyzer::new().analyze(&graph);
        assert!(
            findings.is_empty(),
            "unknown provenance should not trigger leak"
        );
    }

    #[test]
    fn forwarding_node_emits_one_finding_per_edge() {
        // Two sources feeding untrusted data to a forwarding node.
        // The forwarding node is not a boundary source (has untrusted incoming).
        let mut graph = CirGraph::new("lib.rs");
        graph.add_node(node("src1", "read_request", TrustProvenance::Untrusted));
        graph.add_node(node("src2", "read_config", TrustProvenance::Untrusted));
        graph.add_node(node("processor", "process", TrustProvenance::Untrusted));
        graph.add_edge(edge(
            "e1",
            "src1",
            "processor",
            TrustProvenance::Untrusted,
            5,
        ));
        graph.add_edge(edge(
            "e2",
            "src2",
            "processor",
            TrustProvenance::Untrusted,
            10,
        ));

        let findings = BoundaryLeakAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 2, "expected two boundary-leak findings");
    }

    #[test]
    fn trusted_edge_emits_no_finding() {
        let mut graph = CirGraph::new("lib.rs");
        graph.add_node(node("src", "internal", TrustProvenance::Trusted));
        graph.add_node(node("dest", "process", TrustProvenance::Trusted));
        graph.add_edge(edge("e1", "src", "dest", TrustProvenance::Trusted, 5));

        let findings = BoundaryLeakAnalyzer::new().analyze(&graph);
        assert!(findings.is_empty(), "trusted edges should not trigger leak");
    }

    #[test]
    fn boundary_source_identification() {
        let mut graph = CirGraph::new("lib.rs");
        graph.add_node(node("a", "read_request", TrustProvenance::Untrusted)); // source: no untrusted input
        graph.add_node(node("b", "forward", TrustProvenance::Untrusted)); // not source: has untrusted input from a
        graph.add_edge(edge("e1", "a", "b", TrustProvenance::Untrusted, 5));

        let analyzer = BoundaryLeakAnalyzer::new();
        let sources = analyzer.identify_boundary_sources(&graph);
        assert!(
            sources.contains(&sid("a")),
            "'a' should be a boundary source"
        );
        assert!(
            !sources.contains(&sid("b")),
            "'b' should NOT be a boundary source"
        );
    }
}
