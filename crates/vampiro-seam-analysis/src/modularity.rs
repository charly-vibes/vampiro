//! Modularity tracer (REQ-8, REQ-V3–V4, REQ-V7, REQ-C5).
//!
//! Three checks, all emitting on the `modularity` axis (exactly one axis per
//! REQ-4) or as axis-less diagnostics (REQ-V3 enforced-unreachable):
//!
//! 1. **Edge-level reach-through** (REQ-8, REQ-V3, REQ-C5): for each CIR edge,
//!    test whether a morphism from the target's declared scope to the caller's
//!    scope exists in the legitimate subcategory 𝒢 (built from nesting
//!    ancestors + facade/export generators). If not:
//!    - `Enforced` boundary → `boundary:enforced-unreachable` **diagnostic**
//!      (the compiler should have prevented this; a frontend reporting one has
//!      a bug).
//!    - `Advisory` boundary → `modularity` finding classified `reach-through`.
//!    - `EnforcedOpen` + `L4` (facade) → always reachable, no finding.
//!    - `EnforcedOpen` + `< L4` + `internal_by_convention` → REQ-V4
//!      `over-exposure` (declaration-level, see below).
//!
//! 2. **Declaration-level over-exposure** (REQ-V4): a declaration at
//!    `enforced-open` that is marked internal-by-convention (`#[doc(hidden)]`,
//!    leading-underscore, or excluded from the facade) is reachable from
//!    outside its package — the problem is that the item is reachable at all,
//!    not that a caller reached it improperly.
//!
//! 3. **Facade-level facade-leak** (REQ-V7): a facade re-export of a symbol
//!    whose underlying declaration sits at a deeper (more hidden) level than
//!    the facade's own `L4` level.

use std::path::PathBuf;

use vampiro_cir::CirGraph;

use crate::finding::{Diagnostic, Evidence, Finding, Severity};
use vampiro_cir::{BoundaryKind, LatticeLevel, VisibilityFacts};

/// The modularity tracer. See module docs.
#[derive(Debug, Default, Clone)]
pub struct ModularityAnalyzer;

/// The result of a modularity analysis pass: findings (axis = modularity) and
/// diagnostics (no axis).
pub struct ModularityResult {
    /// Modularity findings (reach-through, over-exposure, facade-leak).
    pub findings: Vec<Finding>,
    /// Axis-less diagnostics (enforced-unreachable, etc.).
    pub diagnostics: Vec<Diagnostic>,
}

impl ModularityAnalyzer {
    /// Construct a modularity tracer.
    pub fn new() -> Self {
        Self
    }

    /// Analyze the graph + visibility facts and return modularity findings and
    /// diagnostics.
    pub fn analyze(
        &self,
        graph: &CirGraph,
        vis: &VisibilityFacts,
    ) -> (Vec<Finding>, Vec<Diagnostic>) {
        let mut findings = Vec::new();
        let mut diagnostics = Vec::new();

        // 1. Edge-level reach-through (REQ-8, REQ-V3, REQ-C5).
        for edge in &graph.edges {
            let _caller = graph.node_by_id(&edge.source);
            let Some(target) = graph.node_by_id(&edge.target) else {
                continue;
            };
            let Some(target_vis) = vis.fact_for(&edge.target) else {
                continue;
            };
            let Some(caller_vis) = vis.fact_for(&edge.source) else {
                continue;
            };

            // REQ-C5: is there a morphism from target's scope to caller's scope
            // in 𝒢 (nesting + facade/export generators)?
            let reachable = vis.nesting_reachable(&caller_vis.scope, &target_vis.scope)
                || vis.facade_reachable(&caller_vis.scope, &edge.target)
                || target_vis.level.is_facade();

            if reachable {
                continue;
            }

            // REQ-V3: classify by boundary kind.
            match target_vis.boundary {
                BoundaryKind::Enforced => {
                    // Enforced crossings cannot occur in valid source. If a
                    // frontend reports one, it's a plugin bug → diagnostic.
                    diagnostics.push(Diagnostic {
                        diagnostic: "boundary:enforced-unreachable".into(),
                        path: PathBuf::from(&edge.span.file),
                        line_range: (edge.span.start_line..=edge.span.end_line).into(),
                        detail: format!(
                            "edge to {} at {} ({}) crosses an enforced boundary; \
                             enforced crossings cannot occur in valid source — \
                             the frontend plugin may have a classification bug",
                            target.name.as_deref().unwrap_or("(unnamed)"),
                            target_vis.level,
                            target_vis.boundary,
                        ),
                    });
                }
                BoundaryKind::Advisory => {
                    findings.push(Finding {
                        rule: "REQ-8".into(),
                        path: PathBuf::from(&edge.span.file),
                        line_range: (edge.span.start_line..=edge.span.end_line).into(),
                        severity: Severity::Medium,
                        axis: crate::finding::Axis::Modularity,
                        filtration_distance: None,
                        evidence: Evidence::ReachThrough {
                            target_level: target_vis.level.to_string(),
                            target_boundary: target_vis.boundary.to_string(),
                            boundary_crossed: format!(
                                "{} → {}",
                                target_vis.scope, caller_vis.scope
                            ),
                        },
                        classification: "reach-through".into(),
                    });
                }
                BoundaryKind::EnforcedOpen => {
                    // EnforcedOpen at < L4 with internal_by_convention is
                    // handled by the declaration-level over-exposure check
                    // (REQ-V4) below. An EnforcedOpen declaration at L4 is
                    // the facade itself — always reachable. An EnforcedOpen
                    // declaration at < L4 without internal_by_convention is
                    // technically reachable but not conventionally internal;
                    // it is not a reach-through (the language permits it and
                    // no convention discourages it).
                }
            }
        }

        // 2. Declaration-level over-exposure (REQ-V4).
        for node in &graph.nodes {
            let Some(node_vis) = vis.fact_for(&node.id) else {
                continue;
            };
            if node_vis.boundary == BoundaryKind::EnforcedOpen
                && node_vis.level < LatticeLevel::L4
                && node_vis.internal_by_convention
            {
                // Check whether the node is actually reachable from outside
                // its own file. A pub fn in a private module (L3 EnforcedOpen)
                // is effectively pub(crate) and should not be flagged as
                // over-exposure (vampiro-6ty).
                let cross_file_edge = graph.edges.iter().any(|e| {
                    e.target == node.id
                        && graph
                            .node_by_id(&e.source)
                            .map(|src| src.span.file != node.span.file)
                            .unwrap_or(false)
                });
                if !cross_file_edge {
                    continue;
                }
                findings.push(Finding {
                    rule: "REQ-V4".into(),
                    path: PathBuf::from(&node.span.file),
                    line_range: (node.span.start_line..=node.span.end_line).into(),
                    severity: Severity::Medium,
                    axis: crate::finding::Axis::Modularity,
                    filtration_distance: None,
                    evidence: Evidence::OverExposure {
                        target_level: node_vis.level.to_string(),
                        convention: "doc(hidden) / leading-underscore / excluded from facade"
                            .into(),
                    },
                    classification: "over-exposure".into(),
                });
            }
        }

        // 3. Facade-level facade-leak (REQ-V7).
        for reexport in &vis.facades {
            let Some(underlying_vis) = vis.fact_for(&reexport.underlying_node) else {
                continue;
            };
            if underlying_vis.level.is_hidden_below_facade() {
                let span = graph
                    .node_by_id(&reexport.underlying_node)
                    .map(|n| n.span.clone());
                if let Some(span) = span {
                    findings.push(Finding {
                        rule: "REQ-V7".into(),
                        path: PathBuf::from(&span.file),
                        line_range: (span.start_line..=span.end_line).into(),
                        severity: Severity::Medium,
                        axis: crate::finding::Axis::Modularity,
                        filtration_distance: None,
                        evidence: Evidence::FacadeLeak {
                            facade_scope: reexport.facade_scope.clone(),
                            exported_name: reexport.exported_name.clone(),
                            underlying_level: underlying_vis.level.to_string(),
                        },
                        classification: "facade-leak".into(),
                    });
                }
            }
        }

        (findings, diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vampiro_cir::NodeKind;
    use vampiro_cir::ScalarKind;
    use vampiro_cir::{
        BoundaryKind, FacadeReexport, LatticeLevel, VisibilityFact, VisibilityFacts,
    };
    use vampiro_cir::{
        CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, Provenance, SourceSpan,
        StableId,
    };

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

    fn node(id: &str, line: usize, name: &str) -> CirNode {
        CirNode {
            id: sid(id),
            domain: vampiro_cir::Shape::Scalar(ScalarKind::Unit),
            codomain: vampiro_cir::Shape::Scalar(ScalarKind::Unit),
            effect: EffectChannel::Plain,
            span: span(line),
            name: Some(name.into()),
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

    fn vis_fact(
        node: &str,
        level: LatticeLevel,
        boundary: BoundaryKind,
        scope: &str,
        internal: bool,
    ) -> VisibilityFact {
        VisibilityFact {
            node: sid(node),
            level,
            boundary,
            scope: scope.into(),
            internal_by_convention: internal,
        }
    }

    // --- REQ-8 / REQ-V3: advisory reach-through ---

    #[test]
    fn advisory_crossing_raises_reach_through_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", 1, "caller"));
        graph.add_node(node("callee", 5, "callee"));
        graph.add_edge(edge("e1", "caller", "callee", 3));

        let mut vis = VisibilityFacts::new("0.1.0");
        vis.add_fact(vis_fact(
            "caller",
            LatticeLevel::L4,
            BoundaryKind::Advisory,
            "pkg::caller_mod",
            false,
        ));
        // Callee is advisory L2 in a different scope tree → advisory crossing.
        vis.add_fact(vis_fact(
            "callee",
            LatticeLevel::L2,
            BoundaryKind::Advisory,
            "other::mod",
            false,
        ));

        let (findings, diags) = ModularityAnalyzer::new().analyze(&graph, &vis);
        assert!(
            diags.is_empty(),
            "advisory crossing must not produce a diagnostic"
        );
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.rule, "REQ-8");
        assert_eq!(f.axis, crate::finding::Axis::Modularity);
        assert_eq!(f.classification, "reach-through");
        assert_eq!(f.severity, Severity::Medium);
        let Evidence::ReachThrough {
            target_level,
            target_boundary,
            boundary_crossed: _,
        } = &f.evidence
        else {
            panic!("expected reach-through evidence");
        };
        assert_eq!(target_level, "L2");
        assert_eq!(target_boundary, "advisory");
    }

    // --- REQ-V3: enforced-unreachable diagnostic ---

    #[test]
    fn enforced_crossing_raises_diagnostic_not_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", 1, "caller"));
        graph.add_node(node("callee", 5, "callee"));
        graph.add_edge(edge("e1", "caller", "callee", 3));

        let mut vis = VisibilityFacts::new("0.1.0");
        vis.add_fact(vis_fact(
            "caller",
            LatticeLevel::L4,
            BoundaryKind::EnforcedOpen,
            "pkg::caller_mod",
            false,
        ));
        // Callee is enforced L1 in a different scope → enforced crossing.
        vis.add_fact(vis_fact(
            "callee",
            LatticeLevel::L1,
            BoundaryKind::Enforced,
            "other::mod",
            false,
        ));

        let (findings, diags) = ModularityAnalyzer::new().analyze(&graph, &vis);
        assert!(
            findings.is_empty(),
            "enforced crossing must not produce a finding"
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].diagnostic, "boundary:enforced-unreachable");
    }

    // --- REQ-C5: arbitrary-depth visibility reachability ---

    #[test]
    fn nesting_reachable_no_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", 1, "caller"));
        graph.add_node(node("callee", 5, "callee"));
        graph.add_edge(edge("e1", "caller", "callee", 3));

        let mut vis = VisibilityFacts::new("0.1.0");
        // Deep nesting: caller in a::b::c::d, callee in a → caller is inside.
        vis.add_nesting("pkg::a::b::c::d", "pkg::a::b::c");
        vis.add_nesting("pkg::a::b::c", "pkg::a::b");
        vis.add_nesting("pkg::a::b", "pkg::a");
        vis.add_fact(vis_fact(
            "caller",
            LatticeLevel::L1,
            BoundaryKind::Enforced,
            "pkg::a::b::c::d",
            false,
        ));
        vis.add_fact(vis_fact(
            "callee",
            LatticeLevel::L1,
            BoundaryKind::Enforced,
            "pkg::a",
            false,
        ));

        let (findings, diags) = ModularityAnalyzer::new().analyze(&graph, &vis);
        assert!(
            findings.is_empty(),
            "caller inside callee's scope → no finding"
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn facade_reachable_no_finding() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", 1, "caller"));
        graph.add_node(node("callee", 5, "callee"));
        graph.add_edge(edge("e1", "caller", "callee", 3));

        let mut vis = VisibilityFacts::new("0.1.0");
        vis.add_nesting("other::pkg", "other");
        vis.add_fact(vis_fact(
            "caller",
            LatticeLevel::L4,
            BoundaryKind::Advisory,
            "other::pkg",
            false,
        ));
        // Callee is L4 via facade re-export at crate root.
        vis.add_fact(vis_fact(
            "callee",
            LatticeLevel::L4,
            BoundaryKind::EnforcedOpen,
            "pkg::internal",
            false,
        ));
        vis.add_facade(FacadeReexport {
            facade_scope: "pkg".into(),
            exported_name: "callee".into(),
            underlying_node: sid("callee"),
        });
        vis.add_nesting("other", "pkg"); // caller's tree reaches the facade scope

        let (findings, _diags) = ModularityAnalyzer::new().analyze(&graph, &vis);
        // L4 facade → always reachable → no finding.
        assert!(findings.is_empty(), "L4 facade → reachable → no finding");
    }

    // --- REQ-V4: over-exposure ---

    #[test]
    fn over_exposure_for_doc_hidden_pub() {
        let mut graph = CirGraph::new("src/lib.rs");
        // A pub fn marked doc(hidden) at L3, internal_by_convention = true.
        // This node has an incoming edge from a different file, making it
        // externally reachable (vampiro-6ty).
        graph.add_node(node("exposed", 3, "_internal"));
        // Caller in a different file to trigger the cross-file edge check.
        graph.add_node(CirNode {
            id: sid("caller"),
            domain: vampiro_cir::Shape::Scalar(ScalarKind::Unit),
            codomain: vampiro_cir::Shape::Scalar(ScalarKind::Unit),
            effect: EffectChannel::Plain,
            span: SourceSpan {
                file: "other.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 10,
            },
            name: Some("caller".into()),
            trust_provenance: Default::default(),
            is_test: false,
            kind: NodeKind::Declaration,
            containing_function: None,
        });
        graph.add_edge(edge("e1", "caller", "exposed", 2));

        let mut vis = VisibilityFacts::new("0.1.0");
        vis.add_fact(vis_fact(
            "caller",
            LatticeLevel::L2,
            BoundaryKind::Enforced,
            "other",
            false,
        ));
        vis.add_fact(vis_fact(
            "exposed",
            LatticeLevel::L3,
            BoundaryKind::EnforcedOpen,
            "pkg",
            true,
        ));

        let (findings, _diags) = ModularityAnalyzer::new().analyze(&graph, &vis);
        assert_eq!(
            findings.len(),
            1,
            "cross-file edge makes this externally reachable"
        );
        let f = &findings[0];
        assert_eq!(f.rule, "REQ-V4");
        assert_eq!(f.classification, "over-exposure");
        assert_eq!(f.axis, crate::finding::Axis::Modularity);
        let Evidence::OverExposure { target_level, .. } = &f.evidence else {
            panic!("expected over-exposure evidence");
        };
        assert_eq!(target_level, "L3");
    }

    #[test]
    fn no_over_exposure_for_private_module() {
        // A pub fn in a private module with no cross-file edges should not
        // be flagged as over-exposure (vampiro-6ty).
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("exposed", 3, "extract_graph"));

        let mut vis = VisibilityFacts::new("0.1.0");
        vis.add_fact(vis_fact(
            "exposed",
            LatticeLevel::L3,
            BoundaryKind::EnforcedOpen,
            "pkg",
            true,
        ));

        let (findings, _diags) = ModularityAnalyzer::new().analyze(&graph, &vis);
        assert!(
            findings.is_empty(),
            "pub fn in private module with no cross-file callers should not be over-exposed"
        );
    }

    #[test]
    fn no_over_exposure_for_facade_item() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("exposed", 3, "public_fn"));

        let mut vis = VisibilityFacts::new("0.1.0");
        // L4, not internal_by_convention → no over-exposure.
        vis.add_fact(vis_fact(
            "exposed",
            LatticeLevel::L4,
            BoundaryKind::EnforcedOpen,
            "pkg",
            false,
        ));

        let (findings, _diags) = ModularityAnalyzer::new().analyze(&graph, &vis);
        assert!(findings.is_empty(), "L4 facade item → no over-exposure");
    }

    // --- REQ-V7: facade-leak ---

    #[test]
    fn facade_leak_for_deep_underlying_level() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("leaked", 3, "RawTable"));

        let mut vis = VisibilityFacts::new("0.1.0");
        // Underlying declaration at L2 (package-internal), re-exported at L4 facade.
        vis.add_fact(vis_fact(
            "leaked",
            LatticeLevel::L2,
            BoundaryKind::Advisory,
            "pkg::internal",
            false,
        ));
        vis.add_facade(FacadeReexport {
            facade_scope: "pkg".into(),
            exported_name: "RawTable".into(),
            underlying_node: sid("leaked"),
        });

        let (findings, _diags) = ModularityAnalyzer::new().analyze(&graph, &vis);
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.rule, "REQ-V7");
        assert_eq!(f.classification, "facade-leak");
        let Evidence::FacadeLeak {
            facade_scope,
            exported_name,
            underlying_level,
        } = &f.evidence
        else {
            panic!("expected facade-leak evidence");
        };
        assert_eq!(facade_scope, "pkg");
        assert_eq!(exported_name, "RawTable");
        assert_eq!(underlying_level, "L2");
    }

    #[test]
    fn no_facade_leak_when_underlying_is_l4() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("clean", 3, "PublicApi"));

        let mut vis = VisibilityFacts::new("0.1.0");
        vis.add_fact(vis_fact(
            "clean",
            LatticeLevel::L4,
            BoundaryKind::EnforcedOpen,
            "pkg",
            false,
        ));
        vis.add_facade(FacadeReexport {
            facade_scope: "pkg".into(),
            exported_name: "PublicApi".into(),
            underlying_node: sid("clean"),
        });

        let (findings, _diags) = ModularityAnalyzer::new().analyze(&graph, &vis);
        assert!(findings.is_empty(), "L4 underlying → no facade-leak");
    }

    // --- Exactly-one-axis (REQ-4) ---

    #[test]
    fn all_modularity_findings_use_modularity_axis() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("a", 1, "a"));
        graph.add_node(node("b", 5, "b"));
        graph.add_edge(edge("e1", "a", "b", 3));

        let mut vis = VisibilityFacts::new("0.1.0");
        vis.add_fact(vis_fact(
            "a",
            LatticeLevel::L4,
            BoundaryKind::Advisory,
            "x",
            false,
        ));
        vis.add_fact(vis_fact(
            "b",
            LatticeLevel::L2,
            BoundaryKind::Advisory,
            "y",
            false,
        ));

        let (findings, _diags) = ModularityAnalyzer::new().analyze(&graph, &vis);
        assert!(findings
            .iter()
            .all(|f| f.axis == crate::finding::Axis::Modularity));
    }
}
