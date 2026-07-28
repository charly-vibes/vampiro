//! Composition tracer (REQ-7, REQ-23).
//!
//! For each CIR edge, compares the callee's codomain shape against the
//! caller's domain shape. If the produced shape does not structurally unify
//! with the expected shape, the tracer emits a `composition` finding carrying
//! both shapes side by side (REQ-7). Shapes containing a top-level `Opaque`
//! are excluded from composition-break checking per REQ-23 and never produce a
//! composition finding.
//!
//! Unification is deliberately coarse (EARS §1: "deliberately coarser than a
//! full type") and operates on [`Shape::normalize`](vampiro_cir::Shape)d
//! shapes per the approved canonicalization contract
//! (`docs/decisions/shape-canonicalization.md`).

use vampiro_cir::{CirGraph, Shape};

use crate::finding::Finding;

/// The result of comparing a produced shape against an expected shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unification {
    /// The produced shape admits the expected shape — no finding.
    Match,
    /// The shapes do not unify. `unhandled` lists the produced union arms the
    /// caller left without a handling branch (empty for cross-variant
    /// mismatches where no arm matches).
    Mismatch { unhandled: Vec<Shape> },
    /// Either shape is top-level `Opaque` (REQ-23); composition-break checking
    /// is skipped. The same edge may still be checked on other axes.
    OpaqueExcluded,
}

/// Unify a produced (callee codomain) shape against an expected (caller
/// domain) shape.
///
/// Coarse, structural, normalization-based. See module docs.
pub fn unify_shapes(produced: &Shape, expected: &Shape) -> Unification {
    let produced = produced.normalize();
    let expected = expected.normalize();

    // REQ-23: top-level opaque excludes the edge from composition-break
    // checking. Nested opaque within a non-opaque compound is left to later
    // refinement (degrades only that arm per the canonicalization decision).
    if matches!(produced, Shape::Opaque) || matches!(expected, Shape::Opaque) {
        return Unification::OpaqueExcluded;
    }

    if produced == expected {
        return Unification::Match;
    }

    // Produced union, expected non-union: the caller handles the arms it
    // matches; the rest are unhandled → composition break (the parse_amount
    // case from the EARS worked example).
    if let Shape::Union(arms) = &produced {
        let mut unhandled: Vec<Shape> = Vec::new();
        for arm in arms {
            if !shape_covers(&expected, arm) {
                unhandled.push(arm.clone());
            }
        }
        // If every arm is unhandled, the caller expects something the union
        // does not contain at all — still a mismatch, with all arms as
        // witnesses. If some arms are handled and some are not, only the
        // unhandled ones are witnesses. If none are unhandled, the caller
        // covers every arm → match (fall through to Match below).
        if unhandled.is_empty() {
            // Caller covers every arm.
            // (Union of identical arms collapses to Match only when the
            // caller's expected shape covers all arms.)
            return Unification::Match;
        }
        unhandled.sort_by_key(|s| serde_json::to_string(s).unwrap_or_default());
        unhandled.dedup();
        return Unification::Mismatch { unhandled };
    }

    // Expected union, produced non-union: the caller accepts a sum; the
    // produced value matches at most one arm. No break (the caller handles
    // that arm; the others simply do not occur here).
    if let Shape::Union(_arms) = &expected {
        if shape_covers(&expected, &produced) {
            return Unification::Match;
        }
        return Unification::Mismatch {
            unhandled: Vec::new(),
        };
    }

    // Cross-variant or leaf mismatch.
    Unification::Mismatch {
        unhandled: Vec::new(),
    }
}

/// Does `container` (a union, or a single shape) admit `value`?
fn shape_covers(container: &Shape, value: &Shape) -> bool {
    let container = container.normalize();
    let value = value.normalize();
    match &container {
        Shape::Union(arms) => arms
            .iter()
            .any(|arm| arm == &value || shape_covers(arm, &value)),
        _ => container == value,
    }
}

/// The composition tracer. See module docs.
#[derive(Debug, Default, Clone)]
pub struct CompositionAnalyzer;

impl CompositionAnalyzer {
    /// Construct a composition tracer.
    pub fn new() -> Self {
        Self
    }

    /// Analyze every edge in `graph` and return one composition finding per
    /// edge whose callee codomain does not unify with the caller domain.
    pub fn analyze(&self, graph: &CirGraph) -> Vec<Finding> {
        let mut findings = Vec::new();
        for edge in &graph.edges {
            let Some(callee) = graph.node_by_id(&edge.target) else {
                continue;
            };
            let Some(caller) = graph.node_by_id(&edge.source) else {
                continue;
            };
            let unification = unify_shapes(&callee.codomain, &caller.domain);
            match unification {
                Unification::Match | Unification::OpaqueExcluded => continue,
                Unification::Mismatch { unhandled } => {
                    findings.push(Finding::composition_mismatch(
                        edge.span.file.clone().into(),
                        edge.span.start_line..=edge.span.end_line,
                        caller.domain.clone(),
                        callee.codomain.clone(),
                        unhandled,
                    ));
                }
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vampiro_cir::Shape;

    // --- structural unification (REQ-7) ---

    #[test]
    fn unify_scalar_match() {
        assert_eq!(
            unify_shapes(&Shape::Scalar, &Shape::Scalar),
            Unification::Match
        );
    }

    #[test]
    fn unify_record_order_independent() {
        let a = Shape::Record(vec![Shape::Scalar, Shape::Opaque]);
        let b = Shape::Record(vec![Shape::Opaque, Shape::Scalar]);
        assert_eq!(unify_shapes(&a, &b), Unification::Match);
    }

    #[test]
    fn unify_union_subset_unhandled() {
        // parse_amount case: produced union<Decimal,None>, expected Decimal.
        let produced = Shape::Union(vec![Shape::Scalar, Shape::Opaque]);
        let expected = Shape::Scalar;
        assert_eq!(
            unify_shapes(&produced, &expected),
            Unification::Mismatch {
                unhandled: vec![Shape::Opaque],
            }
        );
    }

    #[test]
    fn unify_union_all_arms_covered_matches() {
        // Caller expects a union that covers every produced arm → match.
        let produced = Shape::Union(vec![Shape::Scalar, Shape::Opaque]);
        let expected = Shape::Union(vec![Shape::Scalar, Shape::Opaque]);
        assert_eq!(unify_shapes(&produced, &expected), Unification::Match);
    }

    #[test]
    fn unify_union_no_arm_matches() {
        let produced = Shape::Union(vec![Shape::Scalar]);
        let expected = Shape::Record(vec![Shape::Scalar]);
        assert_eq!(
            unify_shapes(&produced, &expected),
            Unification::Mismatch {
                unhandled: vec![Shape::Scalar],
            }
        );
    }

    #[test]
    fn unify_expected_union_covers_produced() {
        // Caller accepts union<Decimal,None>; callee produces Decimal → match.
        let produced = Shape::Scalar;
        let expected = Shape::Union(vec![Shape::Scalar, Shape::Opaque]);
        assert_eq!(unify_shapes(&produced, &expected), Unification::Match);
    }

    #[test]
    fn unify_cross_variant_mismatch() {
        assert_eq!(
            unify_shapes(&Shape::Scalar, &Shape::Record(vec![Shape::Scalar])),
            Unification::Mismatch { unhandled: vec![] }
        );
    }

    // --- opaque-shape exclusion (REQ-23) ---

    #[test]
    fn unify_produced_opaque_excluded() {
        assert_eq!(
            unify_shapes(&Shape::Opaque, &Shape::Scalar),
            Unification::OpaqueExcluded
        );
    }

    #[test]
    fn unify_expected_opaque_excluded() {
        assert_eq!(
            unify_shapes(&Shape::Scalar, &Shape::Opaque),
            Unification::OpaqueExcluded
        );
    }

    // --- analyzer end to end over a hand-built CIR graph ---

    use vampiro_cir::{
        CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, Provenance, SourceSpan,
        StableId,
    };

    fn node(id: &str, domain: Shape, codomain: Shape) -> CirNode {
        CirNode {
            id: StableId::new(id),
            domain,
            codomain,
            effect: EffectChannel::Plain,
            span: SourceSpan {
                file: "src/lib.rs".into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 10,
            },
            name: Some(id.into()),
        }
    }

    fn edge(id: &str, source: &str, target: &str, line: usize) -> CirEdge {
        CirEdge {
            id: StableId::new(id),
            source: StableId::new(source),
            target: StableId::new(target),
            resolution: EffectResolution::Propagated,
            unwrap_evidence: None,
            provenance: Provenance::Direct,
            span: SourceSpan {
                file: "src/lib.rs".into(),
                start_line: line,
                start_column: 5,
                end_line: line,
                end_column: 20,
            },
            discard_spans: Vec::new(),
        }
    }

    #[test]
    fn analyze_emits_finding_on_mismatch_with_side_by_side_evidence() {
        let mut graph = CirGraph::new("src/lib.rs");
        // caller expects Decimal (Scalar), callee produces union<Decimal,None>.
        graph.add_node(node("caller", Shape::Scalar, Shape::Scalar));
        graph.add_node(node(
            "callee",
            Shape::Scalar,
            Shape::Union(vec![Shape::Scalar, Shape::Opaque]),
        ));
        graph.add_edge(edge("e1", "caller", "callee", 7));

        let findings = CompositionAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 1, "exactly one composition finding");
        let f = &findings[0];
        assert_eq!(f.rule, "REQ-7");
        assert_eq!(f.axis, crate::finding::Axis::Composition);
        assert_eq!(f.severity, crate::finding::Severity::Medium);
        assert_eq!(f.line_range, crate::finding::LineRange::new(7, 7));

        // Side-by-side evidence (REQ-7).
        #[allow(irrefutable_let_patterns)]
        let crate::finding::Evidence::CompositionMismatch {
            caller_expected,
            callee_produced,
            unhandled,
        } = &f.evidence
        else {
            panic!("expected composition mismatch evidence");
        };
        assert_eq!(caller_expected, &Shape::Scalar);
        assert_eq!(
            callee_produced,
            &Shape::Union(vec![Shape::Opaque, Shape::Scalar])
        );
        assert_eq!(unhandled, &vec![Shape::Opaque]);
    }

    #[test]
    fn analyze_no_finding_on_match() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", Shape::Scalar, Shape::Scalar));
        graph.add_node(node("callee", Shape::Scalar, Shape::Scalar));
        graph.add_edge(edge("e1", "caller", "callee", 7));
        assert!(CompositionAnalyzer::new().analyze(&graph).is_empty());
    }

    #[test]
    fn analyze_opaque_shape_excluded() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node("caller", Shape::Scalar, Shape::Scalar));
        // Callee codomain opaque → excluded by REQ-23.
        graph.add_node(node("callee", Shape::Scalar, Shape::Opaque));
        graph.add_edge(edge("e1", "caller", "callee", 7));
        assert!(
            CompositionAnalyzer::new().analyze(&graph).is_empty(),
            "opaque codomain must not raise a composition finding"
        );
    }
}
