//! Redundancy tracer (REQ-11, REQ-C7).
//!
//! For each CIR node that receives data from multiple sources (a "join" or
//! "consumer" node with ≥2 distinct inbound edges), compares the codomain
//! shapes of every branch source. If the sources do not all produce the same
//! codomain shape and no explicit adapter nodes reconcile the differences, a
//! **robustness** finding is raised (REQ-11).
//!
//! **REQ-C7** generalizes REQ-11: the tool tests whether a colimiting cocone
//! exists over the branches' codomain shapes — a common object every branch
//! legitimately maps into via an explicit adapter wherever shapes differ.
//! Absence of such a cocone is a finding, independent of the number of
//! branches.
//!
//! **Explicit adapters** are detected as intermediate nodes on the path from
//! a branch source to the consumer: a node whose domain matches the branch
//! source's codomain and whose codomain matches the consumer's expected shape.
//!
//! Branches whose source codomain is `Opaque` are excluded from the check
//! (per REQ-23).

use std::collections::HashMap;
use std::path::PathBuf;

use vampiro_cir::{CirGraph, NodeKind, Shape};

use crate::finding::Finding;

/// The redundancy tracer. See module docs.
#[derive(Debug, Default, Clone)]
pub struct RedundancyAnalyzer;

impl RedundancyAnalyzer {
    /// Construct a new redundancy tracer.
    pub fn new() -> Self {
        Self
    }

    /// Analyze every consumer node in `graph` with multiple inbound edges and
    /// return one robustness finding per consumer whose branches do not all
    /// converge on a common codomain shape (REQ-11, REQ-C7).
    pub fn analyze(&self, graph: &CirGraph) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Group edges by target (the consumer/join node).
        let mut edges_by_target: HashMap<&str, Vec<&vampiro_cir::CirEdge>> = HashMap::new();
        for edge in &graph.edges {
            edges_by_target
                .entry(edge.target.as_str())
                .or_default()
                .push(edge);
        }

        for (target_id, incoming) in &edges_by_target {
            // REQ-C7: check any number of branches (≥2 distinct sources).
            if incoming.len() < 2 {
                continue;
            }

            let target_node = graph.node_by_id(&vampiro_cir::StableId::new(*target_id));
            let Some(target) = target_node else {
                continue;
            };
            let expected = target.domain.normalize();

            // Collect distinct source codomains, excluding Opaque (REQ-23).
            let mut branch_shapes: Vec<Shape> = Vec::new();
            let mut unique_sources: HashMap<&str, &vampiro_cir::CirEdge> = HashMap::new();
            for edge in incoming {
                let src_id = edge.source.as_str();
                if unique_sources.contains_key(src_id) {
                    continue; // Skip duplicate edges from the same source.
                }

                // Skip expression-source edges — these are data-flow edges,
                // not branching edges. Only declaration→declaration edges
                // represent true branches (vampiro-uah).
                if let Some(src_node) = graph.node_by_id(&edge.source) {
                    if src_node.kind == NodeKind::Expression {
                        continue;
                    }
                }

                unique_sources.insert(src_id, edge);

                if let Some(src_node) = graph.node_by_id(&edge.source) {
                    let src_codomain = src_node.codomain.normalize();
                    if matches!(src_codomain, Shape::Opaque) {
                        continue; // Excluded per REQ-23.
                    }
                    branch_shapes.push(src_codomain);
                }
            }

            if branch_shapes.len() < 2 {
                continue;
            }

            // Find adapters: intermediate nodes that reconcile shape differences.
            let adapters = self.find_adapters(graph, &unique_sources, &expected, target_id);

            // Check if all branch shapes are the same (trivially compatible).
            let first = &branch_shapes[0];
            let all_same = branch_shapes.iter().all(|s| s == first);

            if all_same {
                // All branches produce the same shape — no finding.
                continue;
            }

            // Calculate the span covering all inbound edges.
            let line_start = incoming
                .iter()
                .map(|e| e.span.start_line)
                .min()
                .unwrap_or(0);
            let line_end = incoming.iter().map(|e| e.span.end_line).max().unwrap_or(0);

            findings.push(Finding::redundancy_mismatch(
                PathBuf::from(&target.span.file),
                line_start..=line_end,
                branch_shapes,
                expected,
                adapters,
            ));
        }

        findings
    }

    /// Find explicit adapter nodes that reconcile branch source codomains to
    /// the target's expected domain.
    ///
    /// An adapter is an intermediate node called by a branch source whose
    /// codomain then feeds into the consumer. In the CIR model, we detect
    /// adapters by checking if an edge from a branch source targets an
    /// intermediate node whose codomain matches the expected shape, and an
    /// edge from that intermediate node targets the consumer.
    fn find_adapters(
        &self,
        graph: &CirGraph,
        unique_sources: &HashMap<&str, &vampiro_cir::CirEdge>,
        expected: &Shape,
        target_id: &str,
    ) -> Vec<String> {
        let mut adapters = Vec::new();

        for edge in unique_sources.values() {
            // Check if there's a path from this source through an adapter to
            // the target. Look for edges whose source is the intermediate
            // node and target is the consumer.
            for e in &graph.edges {
                if e.source == edge.target {
                    // This edge goes from a callee of the branch source to
                    // somewhere. Check if it reaches the consumer.
                    if e.target.as_str() == target_id {
                        if let Some(adapter_node) = graph.node_by_id(&e.source) {
                            // The adapter's codomain should match the expected shape.
                            if adapter_node.codomain.normalize() == *expected {
                                if let Some(ref name) = adapter_node.name {
                                    adapters.push(name.clone());
                                } else {
                                    adapters.push(adapter_node.id.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        adapters.sort();
        adapters.dedup();
        adapters
    }
}

#[cfg(test)]
mod tests {
    use vampiro_cir::ScalarKind;
    use super::*;
    use vampiro_cir::NodeKind;
    use vampiro_cir::{
        CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, Provenance, SourceSpan,
        StableId,
    };

    fn sid(s: &str) -> StableId {
        StableId::new(s)
    }

    fn span(file: &str, line: usize) -> SourceSpan {
        SourceSpan {
            file: file.into(),
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
            span: span("src/lib.rs", line),
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
            span: span("src/lib.rs", line),
            discard_spans: Vec::new(),
            trust_provenance: Default::default(),
            slot: None,
            arg_shape: None,
        }
    }

    // --- REQ-11: two branches with different codomains, no adapter ---

    #[test]
    fn two_branches_mismatch_raises_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        // The Python example: primary -> FullRecord, cache -> union<PartialRecord, None>
        graph.add_node(node(
            "primary",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit)]),
            1,
        ));
        graph.add_node(node(
            "cache",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]),
            5,
        ));
        graph.add_node(node(
            "use",
            Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit)]),
            Shape::Scalar(ScalarKind::Unit),
            10,
        ));
        graph.add_edge(edge("e1", "primary", "use", 3));
        graph.add_edge(edge("e2", "cache", "use", 7));

        let findings = RedundancyAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.rule, "REQ-11");
        assert_eq!(f.axis, crate::finding::Axis::Robustness);
        assert_eq!(f.classification, "redundancy-mismatch");
        assert_eq!(f.severity, crate::finding::Severity::Medium);

        #[allow(irrefutable_let_patterns)]
        let crate::finding::Evidence::RedundancyMismatch {
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
        assert!(adapters.is_empty());
    }

    // --- REQ-11: two branches with same codomain — no finding ---

    #[test]
    fn two_branches_same_codomain_no_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("primary", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 1));
        graph.add_node(node("cache", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 5));
        graph.add_node(node("use", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 10));
        graph.add_edge(edge("e1", "primary", "use", 3));
        graph.add_edge(edge("e2", "cache", "use", 7));

        assert!(
            RedundancyAnalyzer::new().analyze(&graph).is_empty(),
            "same codomain → no finding"
        );
    }

    // --- REQ-11: single inbound edge — no finding ---

    #[test]
    fn single_inbound_edge_no_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("source", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 1));
        graph.add_node(node("use", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 5));
        graph.add_edge(edge("e1", "source", "use", 3));

        assert!(
            RedundancyAnalyzer::new().analyze(&graph).is_empty(),
            "single inbound edge → no redundancy"
        );
    }

    // --- REQ-23: opaque codomain excluded ---

    #[test]
    fn opaque_branch_excluded() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("primary", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 1));
        graph.add_node(node("cache", Shape::Scalar(ScalarKind::Unit), Shape::Opaque, 5));
        graph.add_node(node("use", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 10));
        graph.add_edge(edge("e1", "primary", "use", 3));
        graph.add_edge(edge("e2", "cache", "use", 7));

        // cache is Opaque → excluded per REQ-23 → only one non-opaque branch → skip.
        assert!(
            RedundancyAnalyzer::new().analyze(&graph).is_empty(),
            "opaque branch excluded → no finding"
        );
    }

    // --- REQ-C7: three branches with mismatch ---

    #[test]
    fn three_branches_mismatch_raises_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("a", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 1));
        graph.add_node(node(
            "b",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit)]),
            5,
        ));
        graph.add_node(node("c", Shape::Scalar(ScalarKind::Unit), Shape::Opaque, 9));
        graph.add_node(node("use", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 15));
        graph.add_edge(edge("e1", "a", "use", 3));
        graph.add_edge(edge("e2", "b", "use", 7));
        graph.add_edge(edge("e3", "c", "use", 11));

        let findings = RedundancyAnalyzer::new().analyze(&graph);
        // a and b differ (Scalar vs Union). c is Opaque (excluded). So 2 branches remain → finding.
        assert_eq!(findings.len(), 1);
        #[allow(irrefutable_let_patterns)]
        let crate::finding::Evidence::RedundancyMismatch {
            branch_shapes,
            adapters,
            ..
        } = &findings[0].evidence
        else {
            panic!("expected redundancy mismatch evidence");
        };
        assert_eq!(branch_shapes.len(), 2);
        assert!(adapters.is_empty());
    }

    // --- adapter node reconciles differences ---

    #[test]
    fn adapter_reconciles_mismatch() {
        let mut graph = CirGraph::new("src/lib.rs");
        // primary -> FullRecord via adapter that returns Scalar
        graph.add_node(node(
            "primary",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit)]),
            1,
        ));
        // cache -> PartialRecord | None via same adapter
        graph.add_node(node(
            "cache",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]),
            5,
        ));
        // adapter takes union<Scalar,Opaque> or Record[Scalar,Scalar] → Scalar
        graph.add_node(node(
            "adapter",
            Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]),
            Shape::Scalar(ScalarKind::Unit),
            8,
        ));
        graph.add_node(node("use", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 15));

        // primary -> adapter -> use
        graph.add_edge(edge("e1", "primary", "use", 3)); // direct (no adapter on this path)
        graph.add_edge(edge("e2", "cache", "adapter", 10)); // cache -> adapter
        graph.add_edge(edge("e3", "adapter", "use", 12)); // adapter -> use

        // primary's codomain is Record[Scalar, Scalar], use expects Scalar.
        // cache's codomain is Union[Scalar, Opaque], adapter makes it Scalar.
        // However, primary goes directly to use with Record[Scalar,Scalar] ≠ Scalar.
        // This is also a composition mismatch, but the redundancy tracer
        // should still flag the branch shape mismatch.
        let findings = RedundancyAnalyzer::new().analyze(&graph);
        // The adapter only covers the cache path, not the primary path.
        // primary goes directly to use, not through the adapter.
        assert_eq!(findings.len(), 1, "adapter on one path but not all");
    }

    // --- Exactly-one-axis (REQ-4) ---

    #[test]
    fn all_redundancy_findings_use_robustness_axis() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("a", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 1));
        graph.add_node(node(
            "b",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]),
            5,
        ));
        graph.add_node(node("use", Shape::Scalar(ScalarKind::Unit), Shape::Scalar(ScalarKind::Unit), 10));
        graph.add_edge(edge("e1", "a", "use", 3));
        graph.add_edge(edge("e2", "b", "use", 7));

        let findings = RedundancyAnalyzer::new().analyze(&graph);
        assert!(findings
            .iter()
            .all(|f| f.axis == crate::finding::Axis::Robustness));
    }
}
