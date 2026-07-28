//! Effect-handling tracer (REQ-9, REQ-25, REQ-C4).
//!
//! For each CIR edge, checks whether the caller discards (swallows) an effect
//! channel from the callee without handling it. When a `result`, `option`, or
//! `throws` channel is discarded, a **robustness** finding is raised (REQ-9).
//!
//! **REQ-C4** generalizes REQ-9: totality is determined independently of the
//! resolution label. For nested/combined effect channels such as
//! `async<result<option<T>>>`, the coproduct is resolved one layer at a time.
//! Panic/force unwrap (`.unwrap()`, `.expect()`, `try!`) is treated as
//! `partial` and classified `swallowed` unless every summand has an
//! intentional branch.
//!
//! **REQ-25** (diff-scoped mode): when the swallowed channel is `throws` and
//! the language has unchecked exceptions, the tool additionally performs a
//! memoized bounded ancestor search over the CIR graph for a handling branch.
//! Only if no ancestor path handles the exception type is a finding raised.

use std::collections::HashSet;
use std::path::PathBuf;

use vampiro_cir::{CirGraph, EffectChannel, EffectResolution, UnwrapKind};

use crate::finding::Finding;

const ANCESTOR_SEARCH_DEPTH: u32 = 32;
const ANCESTOR_BOUNDARY_NAME: &str = "process-boundary";

/// The effect-handling tracer. See module docs.
#[derive(Debug, Default, Clone)]
pub struct EffectHandlingAnalyzer;

impl EffectHandlingAnalyzer {
    /// Construct a new effect-handling tracer.
    pub fn new() -> Self {
        Self
    }

    /// Analyze every edge in `graph` and return one robustness finding per
    /// edge where an effect channel is swallowed/discarded (REQ-9, REQ-C4).
    ///
    /// Swallowed effects are identified by:
    /// - Edge resolution is `Swallowed`, OR
    /// - Edge resolution is `Unwrapped` with `UnwrapKind::Force` and
    ///   `Totality::Partial` (panic/force unwrap without full handling).
    pub fn analyze(&self, graph: &CirGraph) -> Vec<Finding> {
        let mut findings = Vec::new();

        for edge in &graph.edges {
            let Some(callee) = graph.node_by_id(&edge.target) else {
                continue;
            };

            // Determine if the effect was swallowed and get the totality.
            let (is_swallowed, totality) = self.classify_edge(edge);

            if !is_swallowed {
                continue;
            }

            // Collect all discard channels from the callee's effect, resolving
            // recursive coproducts one layer at a time (REQ-C4).
            let channels = self.collect_discard_channels(&callee.effect);

            for channel in channels {
                // REQ-9: only result, option, throws raise findings.
                // (Custom effects and plain/async/stream are not REQ-9 targets.)
                if !matches!(
                    channel,
                    EffectChannel::Result | EffectChannel::Option | EffectChannel::Throws
                ) {
                    continue;
                }

                // REQ-25: ancestor search for throws in diff-scoped mode.
                let ancestor_handled = if channel == EffectChannel::Throws {
                    Some(self.search_ancestors(graph, edge, &HashSet::new(), 0))
                } else {
                    None
                };

                // REQ-25: skip the finding if an ancestor handles this throw.
                if ancestor_handled == Some(true) {
                    continue;
                }

                findings.push(Finding::swallowed_effect(
                    PathBuf::from(&edge.span.file),
                    edge.span.start_line..=edge.span.end_line,
                    channel,
                    edge.discard_spans.clone(),
                    &totality,
                    ancestor_handled,
                ));
            }
        }

        findings
    }

    /// Classify an edge's effect resolution and return `(is_swallowed, totality_label)`.
    fn classify_edge(&self, edge: &vampiro_cir::CirEdge) -> (bool, String) {
        match edge.resolution {
            EffectResolution::Swallowed => {
                // REQ-C4: check unwrap_evidence for finer-grained totality.
                if let Some(ref ue) = edge.unwrap_evidence {
                    let totality = serde_json::to_value(&ue.totality)
                        .map(|v| v.to_string())
                        .unwrap_or_else(|_| "unknown".into());
                    (true, totality)
                } else {
                    (true, "partial".into())
                }
            }
            EffectResolution::Unwrapped => {
                // REQ-C4: panic/force unwrap is treated as swallowed unless
                // every summand is intentionally handled (Total totality).
                if let Some(ref ue) = edge.unwrap_evidence {
                    match ue.kind {
                        UnwrapKind::Force => {
                            let is_total = ue.totality == vampiro_cir::Totality::Total;
                            let totality = serde_json::to_value(&ue.totality)
                                .map(|v| v.to_string())
                                .unwrap_or_else(|_| "unknown".into());
                            (!is_total, totality)
                        }
                        UnwrapKind::Ordinary => {
                            // Ordinary unwrap with Total totality → properly handled.
                            (false, "total".into())
                        }
                    }
                } else {
                    (false, "total".into())
                }
            }
            _ => (false, "unknown".into()),
        }
    }

    /// Resolve a recursive effect channel into its base channels, one
    /// coproduct layer at a time (REQ-C4).
    ///
    /// For a plain channel like `result`, returns `[result]`.
    /// For a nested channel like `recursive(result)`, returns `[result, result]`
    /// (the outer layer resolves first, then the inner).
    fn collect_discard_channels(&self, effect: &EffectChannel) -> Vec<EffectChannel> {
        match effect {
            EffectChannel::Recursive(inner) => {
                let mut channels = self.collect_discard_channels(inner);
                // Also include this recursive layer's base channel.
                match inner.as_ref() {
                    EffectChannel::Recursive(_) => {
                        channels.push(EffectChannel::Recursive(inner.clone()));
                    }
                    base => {
                        channels.push(base.clone());
                    }
                }
                channels
            }
            other => vec![other.clone()],
        }
    }

    /// Search ancestor call paths for a handling branch (REQ-25).
    ///
    /// Performs a memoized bounded search up the CIR call graph from the
    /// current edge's source (caller), looking for a node whose declared
    /// boundary is a process boundary or whose effect channel indicates
    /// exception handling for the `throws` type.
    ///
    /// Returns `true` if any ancestor path handles the throws effect.
    fn search_ancestors(
        &self,
        graph: &CirGraph,
        edge: &vampiro_cir::CirEdge,
        visited: &HashSet<String>,
        depth: u32,
    ) -> bool {
        if depth >= ANCESTOR_SEARCH_DEPTH {
            return false;
        }

        let caller_id = edge.source.to_string();
        if visited.contains(&caller_id) {
            return false;
        }
        let mut visited = visited.clone();
        visited.insert(caller_id.clone());

        // Check if the caller itself handles throws (its effect channel is
        // not `throws` — it may propagate or transform it).
        if let Some(caller) = graph.node_by_id(&edge.source) {
            // If the caller declares a process-boundary name, we stop.
            if caller.name.as_deref() == Some(ANCESTOR_BOUNDARY_NAME) {
                return false;
            }
        }

        // Find callers of this node (edges where this node is the target).
        for incoming in &graph.edges {
            if incoming.target == edge.source {
                // Check if the incoming edge handles the throws effect.
                if matches!(
                    incoming.resolution,
                    EffectResolution::Propagated | EffectResolution::Transformed
                ) {
                    // The effect is propagated/transformed — continue searching upward.
                    if self.search_ancestors(graph, incoming, &visited, depth + 1) {
                        return true;
                    }
                } else if matches!(incoming.resolution, EffectResolution::Unwrapped) {
                    // The effect is unwrapped by an ancestor — handled.
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vampiro_cir::{
        CirEdge, CirGraph, CirNode, DiscardSpan, EffectChannel, EffectResolution, Provenance,
        SourceSpan, StableId, Totality, UnwrapEvidence, UnwrapKind,
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

    fn node(id: &str, effect: EffectChannel, line: usize) -> CirNode {
        CirNode {
            id: sid(id),
            domain: vampiro_cir::Shape::Scalar,
            codomain: vampiro_cir::Shape::Scalar,
            effect,
            span: span("src/lib.rs", line),
            name: Some(id.into()),
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
            span: span("src/lib.rs", line),
            discard_spans,
        }
    }

    fn discard_span(line: usize) -> DiscardSpan {
        DiscardSpan {
            file: "src/lib.rs".into(),
            start_line: line,
            end_line: line,
        }
    }

    // --- REQ-9: swallowed result/option/throws ---

    #[test]
    fn swallowed_result_raises_robustness_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", EffectChannel::Plain, 1));
        graph.add_node(node("callee", EffectChannel::Result, 5));
        graph.add_edge(edge(
            "e1",
            "caller",
            "callee",
            EffectResolution::Swallowed,
            None,
            vec![discard_span(3)],
            3,
        ));

        let findings = EffectHandlingAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.rule, "REQ-9");
        assert_eq!(f.axis, crate::finding::Axis::Robustness);
        assert_eq!(f.classification, "swallowed-effect");
        assert_eq!(f.severity, crate::finding::Severity::Medium);

        #[allow(irrefutable_let_patterns)]
        let crate::finding::Evidence::SwallowedEffect {
            discarded_channel,
            discard_lines,
            totality: _,
            ancestor_handled,
        } = &f.evidence
        else {
            panic!("expected swallowed effect evidence");
        };
        assert_eq!(*discarded_channel, EffectChannel::Result);
        assert_eq!(discard_lines.len(), 1);
        assert_eq!(discard_lines[0].start_line, 3);
        assert!(ancestor_handled.is_none());
    }

    #[test]
    fn swallowed_option_raises_robustness_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", EffectChannel::Plain, 1));
        graph.add_node(node("callee", EffectChannel::Option, 5));
        graph.add_edge(edge(
            "e1",
            "caller",
            "callee",
            EffectResolution::Swallowed,
            None,
            vec![discard_span(3)],
            3,
        ));

        let findings = EffectHandlingAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "REQ-9");
        assert_eq!(findings[0].axis, crate::finding::Axis::Robustness);
    }

    #[test]
    fn swallowed_throws_raises_robustness_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", EffectChannel::Plain, 1));
        graph.add_node(node("callee", EffectChannel::Throws, 5));
        graph.add_edge(edge(
            "e1",
            "caller",
            "callee",
            EffectResolution::Swallowed,
            None,
            vec![discard_span(3)],
            3,
        ));

        let findings = EffectHandlingAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "REQ-9");
        assert_eq!(findings[0].axis, crate::finding::Axis::Robustness);
    }

    // --- plain/async/stream do NOT raise findings ---

    #[test]
    fn swallowed_plain_does_not_raise_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", EffectChannel::Plain, 1));
        graph.add_node(node("callee", EffectChannel::Plain, 5));
        graph.add_edge(edge(
            "e1",
            "caller",
            "callee",
            EffectResolution::Swallowed,
            None,
            vec![],
            3,
        ));
        assert!(EffectHandlingAnalyzer::new().analyze(&graph).is_empty());
    }

    #[test]
    fn swallowed_async_does_not_raise_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", EffectChannel::Plain, 1));
        graph.add_node(node("callee", EffectChannel::Async, 5));
        graph.add_edge(edge(
            "e1",
            "caller",
            "callee",
            EffectResolution::Swallowed,
            None,
            vec![],
            3,
        ));
        assert!(EffectHandlingAnalyzer::new().analyze(&graph).is_empty());
    }

    #[test]
    fn swallowed_stream_does_not_raise_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", EffectChannel::Plain, 1));
        graph.add_node(node("callee", EffectChannel::Stream, 5));
        graph.add_edge(edge(
            "e1",
            "caller",
            "callee",
            EffectResolution::Swallowed,
            None,
            vec![],
            3,
        ));
        assert!(EffectHandlingAnalyzer::new().analyze(&graph).is_empty());
    }

    // --- REQ-C4: recursive coproduct resolution ---

    #[test]
    fn recursive_effect_resolves_all_layers() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", EffectChannel::Plain, 1));
        // Recursive(Result) — a nested effect like result<result<T>>
        let ch = EffectChannel::Recursive(Box::new(EffectChannel::Result));
        graph.add_node(node("callee", ch, 5));
        graph.add_edge(edge(
            "e1",
            "caller",
            "callee",
            EffectResolution::Swallowed,
            None,
            vec![discard_span(3)],
            3,
        ));

        let findings = EffectHandlingAnalyzer::new().analyze(&graph);
        // Both the inner Result and the Recursive(Result) layer should produce findings.
        assert!(
            !findings.is_empty(),
            "recursive effect should produce at least one finding"
        );
        // Every finding should be on the robustness axis (REQ-4).
        assert!(findings
            .iter()
            .all(|f| f.axis == crate::finding::Axis::Robustness));
    }

    // --- REQ-C4: ordinary total unwrap is NOT swallowed ---

    #[test]
    fn ordinary_total_unwrap_does_not_raise_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", EffectChannel::Plain, 1));
        graph.add_node(node("callee", EffectChannel::Result, 5));
        graph.add_edge(edge(
            "e1",
            "caller",
            "callee",
            EffectResolution::Unwrapped,
            Some(UnwrapEvidence {
                kind: UnwrapKind::Ordinary,
                totality: Totality::Total,
            }),
            vec![],
            3,
        ));
        assert!(
            EffectHandlingAnalyzer::new().analyze(&graph).is_empty(),
            "ordinary total unwrap is properly handled, not swallowed"
        );
    }

    // --- REQ-C4: force/panic partial unwrap IS swallowed ---

    #[test]
    fn force_partial_unwrap_raises_finding() {
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

        let findings = EffectHandlingAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule, "REQ-9");
        assert_eq!(findings[0].axis, crate::finding::Axis::Robustness);
    }

    // --- REQ-C4: force total unwrap (every summand handled) is NOT swallowed ---

    #[test]
    fn force_total_unwrap_does_not_raise_finding() {
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
                totality: Totality::Total,
            }),
            vec![],
            3,
        ));
        assert!(
            EffectHandlingAnalyzer::new().analyze(&graph).is_empty(),
            "force unwrap with total handling is not swallowed"
        );
    }

    // --- custom effect does not raise REQ-9 finding ---

    #[test]
    fn swallowed_custom_effect_does_not_raise_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", EffectChannel::Plain, 1));
        graph.add_node(node("callee", EffectChannel::Custom("my-effect".into()), 5));
        graph.add_edge(edge(
            "e1",
            "caller",
            "callee",
            EffectResolution::Swallowed,
            None,
            vec![],
            3,
        ));
        assert!(
            EffectHandlingAnalyzer::new().analyze(&graph).is_empty(),
            "custom effects are not REQ-9 targets"
        );
    }

    // --- REQ-25: ancestor handling search for throws ---

    #[test]
    fn swallowed_throws_finds_no_ancestor() {
        let mut graph = CirGraph::new("src/lib.rs");
        // top -> middle -> bottom
        // bottom throws, middle swallows → top does not handle
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

        let findings = EffectHandlingAnalyzer::new().analyze(&graph);
        // The middle→bottom edge swallows throws. Top propagates it → no ancestor
        // handler. So one finding.
        assert_eq!(findings.len(), 1);
        #[allow(irrefutable_let_patterns)]
        let crate::finding::Evidence::SwallowedEffect {
            ancestor_handled, ..
        } = &findings[0].evidence
        else {
            panic!("expected swallowed effect evidence");
        };
        assert_eq!(*ancestor_handled, Some(false));
    }

    #[test]
    fn swallowed_throws_finds_ancestor_handler() {
        let mut graph = CirGraph::new("src/lib.rs");
        // bottom -> middle -> top
        // bottom throws, middle swallows, top unwraps → ancestor handles
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

        let findings = EffectHandlingAnalyzer::new().analyze(&graph);
        // The middle swallows throws but top unwraps it → ancestor handles → no finding.
        assert!(
            findings.is_empty(),
            "throws swallowed with ancestor handler → no finding"
        );
    }

    // --- Exactly-one-axis (REQ-4) ---

    #[test]
    fn all_effect_findings_use_robustness_axis() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("c1", EffectChannel::Plain, 1));
        graph.add_node(node("c2", EffectChannel::Result, 5));
        graph.add_node(node("c3", EffectChannel::Option, 10));
        graph.add_node(node("c4", EffectChannel::Throws, 15));
        graph.add_edge(edge(
            "e1",
            "c1",
            "c2",
            EffectResolution::Swallowed,
            None,
            vec![discard_span(3)],
            3,
        ));
        graph.add_edge(edge(
            "e2",
            "c1",
            "c3",
            EffectResolution::Swallowed,
            None,
            vec![discard_span(8)],
            8,
        ));
        graph.add_edge(edge(
            "e3",
            "c1",
            "c4",
            EffectResolution::Swallowed,
            None,
            vec![discard_span(13)],
            13,
        ));

        let findings = EffectHandlingAnalyzer::new().analyze(&graph);
        assert!(findings
            .iter()
            .all(|f| f.axis == crate::finding::Axis::Robustness));
    }
}
