//! Composition tracer (REQ-7, REQ-23).
//!
//! For each CIR edge, compares the caller's return type (codomain) against the
//! callee's return type (codomain). If the caller claims to return type X but
//! calls a function returning Y ≠ X (and Y is not opaque), a composition
//! finding is raised carrying both codomains side by side (REQ-7).
//!
//! This catches composition breaks at the **return boundary**: a function
//! whose declared return type contradicts what its callees actually produce.
//! The `parse_amount→apply_discount` seam (comparing a value shape against the
//! callee's expected parameter shape) is handled by the **slot-boundary check**:
//! for edges with a known `slot`, the caller's codomain is compared against
//! `callee.domain[slot]` using the same `unify_shapes` primitive (see
//! [`domain_slot`](vampiro_cir::Shape::domain_slot)).
//!
//! Shapes containing a top-level `Opaque` are excluded from composition-break
//! checking per REQ-23 and never produce a composition finding.
//!
//! Unification is deliberately coarse (EARS §1) and operates on
//! [`Shape::normalize`](vampiro_cir::Shape)d shapes per the approved
//! canonicalization contract.

use vampiro_cir::{CirGraph, NodeKind, ScalarKind, Shape, Totality, UnwrapKind};

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
    // checking. Nested opaque within a non-opaque compound is refined below
    // (see `differs_only_by_unknown`).
    if matches!(produced, Shape::Opaque) || matches!(expected, Shape::Opaque) {
        return Unification::OpaqueExcluded;
    }

    // Bottom (the `!` type, or an uninferred leaf) coerces to / unifies with
    // anything — treat like opaque rather than a break.
    if matches!(produced, Shape::Bottom) || matches!(expected, Shape::Bottom) {
        return Unification::OpaqueExcluded;
    }

    if produced == expected {
        return Unification::Match;
    }

    // --- Fine-grained scalar matching ---
    //
    // Same-kind scalars match; different-kind scalars don't. The generic
    // Unit kind matches itself but NOT other kinds (it represents an
    // unknown/fallback scalar; treat it as distinct from known types).
    match (&produced, &expected) {
        (Shape::Scalar(p_kind), Shape::Scalar(e_kind)) if p_kind == e_kind => {
            return Unification::Match;
        }
        (Shape::Scalar(_), Shape::Scalar(_)) => {
            // Different scalar kinds → mismatch.
            return Unification::Mismatch { unhandled: vec![] };
        }
        _ => {}
    }

    // Scalar(String) matches Ref(Scalar(String)) — &str parameter accepts a
    // String value (Rust auto-refs). Also handles Unit as fallback.
    if let Shape::Scalar(kind) = &produced {
        if let Shape::Ref(inner) = &expected {
            if let Shape::Scalar(inner_kind) = inner.as_ref() {
                if kind == inner_kind {
                    return Unification::Match;
                }
            }
        }
    }

    // Deref coercion (vampiro-224.13): `&Vec<T>` / `&mut Vec<T>` coerce to
    // `&[T]`, `&mut T` to `&T` — both sides keep their Ref, so compare the
    // referents recursively (which picks up the Vec↔slice aliasing rule and
    // everything else). The mut bit is not modeled; a `&mut T` passed where
    // `&T` is expected is a valid coercion. Deref-to-non-Ref is intentionally
    // NOT handled: rustc does not auto-deref in argument position, so a Ref
    // against a non-Ref expected shape stays a genuine mismatch.
    if let (Shape::Ref(p_inner), Shape::Ref(e_inner)) = (&produced, &expected) {
        return unify_shapes(p_inner, e_inner);
    }

    // Parameterized base aliasing: Vec[T] ↔ slice[T] are structurally
    // compatible when their type parameters unify.
    if let (
        Shape::Parameterized {
            base: b1,
            parameters: p1,
        },
        Shape::Parameterized {
            base: b2,
            parameters: p2,
        },
    ) = (&produced, &expected)
    {
        let bases_match = (b1 == "Vec" && b2 == "slice") || (b1 == "slice" && b2 == "Vec");
        if bases_match && p1.len() == p2.len() {
            let all_params_match = p1
                .iter()
                .zip(p2.iter())
                .all(|(l, r)| matches!(unify_shapes(l, r), Unification::Match));
            if all_params_match {
                return Unification::Match;
            }
        }
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

    // Nested opaque exclusion: when two compound shapes are structurally
    // comparable and every differing sub-position involves an unknown
    // (`Opaque`) on at least one side, the edge cannot be judged — exclude it
    // instead of firing a break (post-224.3 the frontend emits Opaque for
    // every unresolvable named type, so this is the common foreign-code case).
    if differs_only_by_unknown(&produced, &expected) {
        return Unification::OpaqueExcluded;
    }

    // Cross-variant or leaf mismatch.
    Unification::Mismatch {
        unhandled: Vec::new(),
    }
}

/// True when `a` and `b` are structurally comparable (same compound variant,
/// same arity) and at least one sub-position differs while every differing
/// sub-position has `Opaque` on at least one side. Leaves that differ without
/// opaque involvement return `false` — those are genuine mismatches.
fn differs_only_by_unknown(a: &Shape, b: &Shape) -> bool {
    match (a, b) {
        (Shape::Opaque, _) | (_, Shape::Opaque) => true,
        (Shape::Bottom, _) | (_, Shape::Bottom) => true,
        // Same-base positional comparison must run before the effect-wrapper
        // arms below, so resolvable differences still fire as mismatches.
        // `Result<T>` (type alias with defaulted error) vs `Result<T, E>`:
        // the extra parameter is the error channel, auto-converted by `?` in
        // compiling code — not a break. Common positions must agree.
        (
            Shape::Parameterized {
                base: b1,
                parameters: p1,
                ..
            },
            Shape::Parameterized {
                base: b2,
                parameters: p2,
                ..
            },
        ) if b1 == b2 => {
            let position_differs = p1
                .iter()
                .zip(p2.iter())
                .any(|(l, r)| l != r && differs_only_by_unknown(l, r));
            let arity_alias = p1.len() != p2.len()
                && (b1 == "Result" || b1 == "Option")
                && p1
                    .iter()
                    .zip(p2.iter())
                    .all(|(l, r)| l == r || differs_only_by_unknown(l, r));
            position_differs || arity_alias
        }
        // Effect wrapper with an unknown parameter vs a different-shape /
        // cross-variant operand: the wrapper's content cannot be judged
        // (e.g. a `?`-unwrapped callee value vs a caller codomain
        // `Result<PathBuf>` with Opaque inner).
        (Shape::Parameterized { base, parameters }, _)
            if (base == "Result" || base == "Option")
                && parameters.iter().any(involves_unknown) =>
        {
            true
        }
        (_, Shape::Parameterized { base, parameters })
            if (base == "Result" || base == "Option")
                && parameters.iter().any(involves_unknown) =>
        {
            true
        }
        (
            Shape::Parameterized {
                base: _b1,
                parameters: p1,
                ..
            },
            Shape::Parameterized {
                base: _b2,
                parameters: p2,
                ..
            },
        ) if p1.len() == p2.len() => {
            let position_differs = p1
                .iter()
                .zip(p2.iter())
                .any(|(l, r)| l != r && differs_only_by_unknown(l, r));
            let bases_differ_with_unknown_params = _b1 != _b2 && p1.iter().any(involves_unknown);
            position_differs || bases_differ_with_unknown_params
        }
        (Shape::Record(f1), Shape::Record(f2)) if f1.len() == f2.len() => {
            f1.iter().zip(f2.iter()).any(|(l, r)| {
                if l == r {
                    false
                } else {
                    differs_only_by_unknown(l, r)
                }
            })
        }
        (Shape::Ref(x), Shape::Ref(y)) => x != y && differs_only_by_unknown(x, y),
        _ => false,
    }
}

/// Does this shape contain an unknown leaf (`Opaque`/`Bottom`) anywhere?
fn involves_unknown(s: &Shape) -> bool {
    match s {
        Shape::Opaque | Shape::Bottom => true,
        Shape::Parameterized { parameters, .. }
        | Shape::Record(parameters)
        | Shape::Union(parameters) => parameters.iter().any(involves_unknown),
        Shape::Ref(inner) => involves_unknown(inner),
        _ => false,
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

/// Extract the inner type from an effect wrapper for `?` operator semantics.
///
/// When a `Result<T, E>` or `Option<T>` is unwrapped via the `?` operator,
/// the value that flows to the caller is `T` (the success path). This helper
/// extracts `T` from the outer `Parameterized` shape for composition comparison.
fn unwrap_outer_effect(shape: &Shape) -> Option<&Shape> {
    match shape {
        Shape::Parameterized { base, parameters } => {
            if base == "Result" || base == "Option" {
                parameters.first()
            } else {
                None
            }
        }
        _ => None,
    }
}

/// vampiro-s3e: are both ends of this edge Rust source? A same-language
/// Rust boundary is one where rustc already proves return-shape contracts,
/// so compiler-subsumed checks (unit codomain) must not fire.
fn same_language_rust(caller_file: &str, callee_file: &str) -> bool {
    caller_file.ends_with(".rs") && callee_file.ends_with(".rs")
}

/// The composition tracer. See module docs.
#[derive(Debug, Default, Clone)]
pub struct CompositionAnalyzer;

impl CompositionAnalyzer {
    /// Construct a composition tracer.
    pub fn new() -> Self {
        Self
    }

    /// Analyze every edge in `graph` and return composition findings.
    ///
    /// Two independent checks run on every edge:
    ///
    /// 1. **Return-boundary check** (REQ-7): compares the callee's codomain
    ///    against the caller's codomain. Catches the case where a caller
    ///    claims to return X but a callee produces Y ≠ X (the original
    ///    composition check).
    /// 2. **Slot-boundary check** (REQ-7 extension): for edges with a known
    ///    argument slot, compares the caller's codomain (the value flowing
    ///    in) against the callee's expected domain at that slot. This catches
    ///    the `parse_amount→apply_discount` seam where a value flows into a
    ///    wrong-shaped parameter.
    pub fn analyze(&self, graph: &CirGraph) -> Vec<Finding> {
        let mut findings = Vec::new();
        for edge in &graph.edges {
            let Some(callee) = graph.node_by_id(&edge.target) else {
                continue;
            };
            let Some(source) = graph.node_by_id(&edge.source) else {
                continue;
            };

            match source.kind {
                NodeKind::Expression => {
                    // --- Data-flow check (vampiro-uah) ---
                    //
                    // Expression-source edges represent data flowing from an
                    // intermediate expression into a callee parameter. Compare
                    // the expression's shape (source.codomain) against the
                    // callee's expected domain at the edge's slot.
                    //
                    // This is the precise comparison that replaces the noisy
                    // return-boundary check for argument-level data flow.
                    if let Some(slot) = edge.slot {
                        if let Some(expected) = callee.domain.domain_slot(slot) {
                            if !matches!(expected, Shape::Opaque) {
                                let value_shape = &source.codomain;
                                match unify_shapes(value_shape, expected) {
                                    Unification::Match | Unification::OpaqueExcluded => {}
                                    Unification::Mismatch { .. } => {
                                        findings.push(Finding::slot_mismatch(
                                            edge.span.file.clone().into(),
                                            edge.span.start_line..=edge.span.end_line,
                                            slot,
                                            expected.clone(),
                                            value_shape.clone(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                NodeKind::Declaration => {
                    // --- Return-boundary check ---
                    //
                    // vampiro-224.9: fire only for return-position calls.
                    // The check compares callee codomain vs caller codomain,
                    // which is meaningful only when the callee's result
                    // actually flows to the caller's return (tail
                    // expression / return operand, tagged by the frontend
                    // as `return_position`). Statement-position, let-bound,
                    // and argument-slot calls are definitionally FPs on
                    // compiling code; their data flow is covered by the
                    // slot-boundary and data-flow checks instead.
                    //
                    // Skip edges where the caller has a void/unit return type
                    // (Scalar as the unit type). There's no composition
                    // contract at the return boundary for void-returning
                    // functions, and the coarse Shape model already cannot
                    // distinguish different scalar types (u32, f64, bool are
                    // all `Scalar`), so this guard loses no precision while
                    // eliminating noise from unrelated call edges.
                    if edge.return_position && source.codomain != Shape::Scalar(ScalarKind::Unit) {
                        // For edges with Ordinary+Total unwrap evidence
                        // (e.g., `?` operator on Result/Option), the callee's
                        // effect wrapper is removed at the call site. Compare
                        // the unwrapped shape (the inner type parameter)
                        // instead of the full effect-wrapped codomain
                        // (vampiro-0j8).
                        let callee_shape = if let Some(ref ue) = edge.unwrap_evidence {
                            if ue.kind == UnwrapKind::Ordinary && ue.totality == Totality::Total {
                                unwrap_outer_effect(&callee.codomain).unwrap_or(&callee.codomain)
                            } else {
                                &callee.codomain
                            }
                        } else {
                            &callee.codomain
                        };
                        // vampiro-s3e: on a same-language Rust boundary, a
                        // Scalar(Unit) callee codomain is the default /
                        // unconstrained codomain, not Mismatch evidence —
                        // rustc proves real unit-vs-value return breaks on
                        // compiling Rust code by construction. The gate keys
                        // on the RAW callee codomain: a `Result<T, E>` callee
                        // with an unresolved `T` (post-unwrap `Scalar(Unit)`)
                        // is an unresolved parameter, not a unit return, and
                        // must still be compared. The gate is scoped to
                        // same-language (`.rs` caller + `.rs` callee) edges:
                        // at cross-language or trust boundaries rustc protects
                        // nothing, so the comparison still fires there.
                        let unit_callee_gated = callee.codomain == Shape::Scalar(ScalarKind::Unit)
                            && same_language_rust(&edge.span.file, &callee.span.file);
                        if !unit_callee_gated {
                            let unification = unify_shapes(callee_shape, &source.codomain);
                            if let Unification::Mismatch { unhandled } = unification {
                                findings.push(Finding::composition_mismatch(
                                    edge.span.file.clone().into(),
                                    edge.span.start_line..=edge.span.end_line,
                                    source.codomain.clone(),
                                    callee.codomain.clone(),
                                    unhandled,
                                ));
                            }
                        }
                    }

                    // --- Slot-boundary check (backward compat) ---
                    //
                    // For edges with a known slot, compare the argument value
                    // shape (computed by the frontend from the argument
                    // expression) against the callee's expected domain at that
                    // slot. Only fires when the frontend could statically
                    // determine the argument's shape via `arg_shape`.
                    //
                    // This is the legacy path for graphs serialized before
                    // data-flow nodes were introduced. New graphs use
                    // expression-source edges instead.
                    if let Some(slot) = edge.slot {
                        if let Some(expected) = callee.domain.domain_slot(slot) {
                            if !matches!(expected, Shape::Opaque) {
                                if let Some(ref value_shape) = edge.arg_shape {
                                    match unify_shapes(value_shape, expected) {
                                        Unification::Match | Unification::OpaqueExcluded => {}
                                        Unification::Mismatch { .. } => {
                                            findings.push(Finding::slot_mismatch(
                                                edge.span.file.clone().into(),
                                                edge.span.start_line..=edge.span.end_line,
                                                slot,
                                                expected.clone(),
                                                value_shape.clone(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vampiro_cir::{ScalarKind, Shape};

    // --- structural unification (REQ-7) ---

    #[test]
    fn unify_scalar_match() {
        assert_eq!(
            unify_shapes(
                &Shape::Scalar(ScalarKind::Unit),
                &Shape::Scalar(ScalarKind::Unit)
            ),
            Unification::Match
        );
    }

    #[test]
    fn unify_record_order_independent() {
        let a = Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]);
        let b = Shape::Record(vec![Shape::Opaque, Shape::Scalar(ScalarKind::Unit)]);
        assert_eq!(unify_shapes(&a, &b), Unification::Match);
    }

    #[test]
    fn unify_union_subset_unhandled() {
        // parse_amount case: produced union<Decimal,None>, expected Decimal.
        let produced = Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]);
        let expected = Shape::Scalar(ScalarKind::Unit);
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
        let produced = Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]);
        let expected = Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]);
        assert_eq!(unify_shapes(&produced, &expected), Unification::Match);
    }

    #[test]
    fn unify_union_no_arm_matches() {
        let produced = Shape::Union(vec![Shape::Scalar(ScalarKind::Unit)]);
        let expected = Shape::Record(vec![Shape::Scalar(ScalarKind::Unit)]);
        assert_eq!(
            unify_shapes(&produced, &expected),
            Unification::Mismatch {
                unhandled: vec![Shape::Scalar(ScalarKind::Unit)],
            }
        );
    }

    #[test]
    fn unify_expected_union_covers_produced() {
        // Caller accepts union<Decimal,None>; callee produces Decimal → match.
        let produced = Shape::Scalar(ScalarKind::Unit);
        let expected = Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]);
        assert_eq!(unify_shapes(&produced, &expected), Unification::Match);
    }

    // --- Deref coercion (vampiro-224.13) ---

    fn vec_int() -> Shape {
        Shape::Parameterized {
            base: "Vec".to_string(),
            parameters: vec![Shape::Scalar(ScalarKind::Int)],
        }
    }

    fn slice_int() -> Shape {
        Shape::Parameterized {
            base: "slice".to_string(),
            parameters: vec![Shape::Scalar(ScalarKind::Int)],
        }
    }

    /// `&mut Vec<T>` / `&Vec<T>` passed where `&[T]` is expected (the
    /// testaruda engine.rs FP): both refs deref-coerce to the same slice.
    #[test]
    fn unify_ref_vec_vs_ref_slice_matches() {
        assert_eq!(
            unify_shapes(
                &Shape::Ref(Box::new(vec_int())),
                &Shape::Ref(Box::new(slice_int()))
            ),
            Unification::Match
        );
    }

    /// Deref coercion direction only: covered by Ref-Ref recursion —
    /// `&Vec<T>` coerces to `&[T]`, never to bare `[T]` in argument position.
    #[test]
    fn unify_ref_produced_derefs_to_expected_stays_mismatch() {
        assert_eq!(
            unify_shapes(&Shape::Ref(Box::new(vec_int())), &slice_int()),
            Unification::Mismatch { unhandled: vec![] }
        );
    }

    /// Deref coercion must not mask genuine mismatches: different referents
    /// still break, both through refs and through the deref path.
    #[test]
    fn unify_ref_mismatch_preserved_for_genuinely_different_referents() {
        let record = Shape::Record(vec![Shape::Scalar(ScalarKind::String)]);
        assert_eq!(
            unify_shapes(
                &Shape::Ref(Box::new(vec_int())),
                &Shape::Ref(Box::new(record.clone()))
            ),
            Unification::Mismatch { unhandled: vec![] }
        );
        assert_eq!(
            unify_shapes(&Shape::Ref(Box::new(vec_int())), &record),
            Unification::Mismatch { unhandled: vec![] }
        );
    }

    #[test]
    fn unify_cross_variant_mismatch() {
        assert_eq!(
            unify_shapes(
                &Shape::Scalar(ScalarKind::Unit),
                &Shape::Record(vec![Shape::Scalar(ScalarKind::Unit)])
            ),
            Unification::Mismatch { unhandled: vec![] }
        );
    }

    // --- opaque-shape exclusion (REQ-23) ---

    #[test]
    fn unify_produced_opaque_excluded() {
        assert_eq!(
            unify_shapes(&Shape::Opaque, &Shape::Scalar(ScalarKind::Unit)),
            Unification::OpaqueExcluded
        );
    }

    #[test]
    fn unify_expected_opaque_excluded() {
        assert_eq!(
            unify_shapes(&Shape::Scalar(ScalarKind::Unit), &Shape::Opaque),
            Unification::OpaqueExcluded
        );
    }

    // --- nested opaque exclusion (vampiro-224.3 follow-up, dogfood-5) ---

    #[test]
    fn unify_nested_opaque_in_same_base_excluded() {
        // Vec<Opaque> vs Vec<Int> — inner type unknown on one side,
        // the edge cannot be judged → excluded, not a break.
        assert_eq!(
            unify_shapes(
                &Shape::Parameterized {
                    base: "Vec".into(),
                    parameters: vec![Shape::Opaque],
                },
                &Shape::Parameterized {
                    base: "Vec".into(),
                    parameters: vec![Shape::Scalar(ScalarKind::Int)],
                }
            ),
            Unification::OpaqueExcluded
        );
    }

    #[test]
    fn unify_nested_opaque_across_bases_excluded() {
        // Option<Opaque> vs Vec<Opaque> — both parameters unknown, the base
        // difference alone is not evidence of a break.
        assert_eq!(
            unify_shapes(
                &Shape::Parameterized {
                    base: "Option".into(),
                    parameters: vec![Shape::Opaque],
                },
                &Shape::Parameterized {
                    base: "Vec".into(),
                    parameters: vec![Shape::Opaque],
                }
            ),
            Unification::OpaqueExcluded
        );
    }

    #[test]
    fn unify_nested_opaque_with_resolvable_diff_still_mismatch() {
        // Result<Int, Opaque> vs Result<Unit, Opaque>: the success parameter
        // differs resolvably on both sides — genuine mismatch preserved.
        let with = |succ: Shape| Shape::Parameterized {
            base: "Result".into(),
            parameters: vec![succ, Shape::Opaque],
        };
        assert_eq!(
            unify_shapes(
                &with(Shape::Scalar(ScalarKind::Int)),
                &with(Shape::Scalar(ScalarKind::Unit))
            ),
            Unification::Mismatch { unhandled: vec![] }
        );
    }

    #[test]
    fn unify_fully_resolved_param_diff_still_mismatch() {
        // Vec<Int> vs Vec<String> — both sides resolved, real mismatch.
        let with = |p: Shape| Shape::Parameterized {
            base: "Vec".into(),
            parameters: vec![p],
        };
        assert_eq!(
            unify_shapes(
                &with(Shape::Scalar(ScalarKind::Int)),
                &with(Shape::Scalar(ScalarKind::String))
            ),
            Unification::Mismatch { unhandled: vec![] }
        );
    }

    #[test]
    fn unify_bottom_excluded() {
        // Bottom (! / uninferred) is compatible with everything.
        assert_eq!(
            unify_shapes(&Shape::Bottom, &Shape::Scalar(ScalarKind::Int)),
            Unification::OpaqueExcluded
        );
        assert_eq!(
            unify_shapes(
                &Shape::Parameterized {
                    base: "Vec".into(),
                    parameters: vec![Shape::Opaque],
                },
                &Shape::Bottom
            ),
            Unification::OpaqueExcluded
        );
    }

    #[test]
    fn unify_leaf_vs_effect_wrapper_with_unknown_param_excluded() {
        // `?`-unwrapped callee value (String) vs caller codomain
        // Result<PathBuf> where PathBuf is Opaque — the wrapper's content is
        // unknown, no judgment possible (dogfood-5 wai mod.rs:319 class).
        assert_eq!(
            unify_shapes(
                &Shape::Scalar(ScalarKind::String),
                &Shape::Parameterized {
                    base: "Result".into(),
                    parameters: vec![Shape::Opaque],
                }
            ),
            Unification::OpaqueExcluded
        );
    }

    #[test]
    fn unify_result_arity_alias_excluded() {
        // Result<T> (alias with defaulted error) vs Result<T, E>: the extra
        // parameter is the error channel, auto-converted by `?` (dogfood-5
        // espectacular doctor.rs:686 class).
        assert_eq!(
            unify_shapes(
                &Shape::Parameterized {
                    base: "Result".into(),
                    parameters: vec![Shape::Scalar(ScalarKind::Unit)],
                },
                &Shape::Parameterized {
                    base: "Result".into(),
                    parameters: vec![
                        Shape::Scalar(ScalarKind::Unit),
                        Shape::Scalar(ScalarKind::String),
                    ],
                }
            ),
            Unification::OpaqueExcluded
        );
    }

    // --- coarse model: Scalar matches Ref(Scalar) (vampiro-51v.3) ---

    #[test]
    fn unify_scalar_matches_ref_scalar() {
        // String literal (Scalar) passed where &str (Ref(Scalar)) expected.
        assert_eq!(
            unify_shapes(
                &Shape::Scalar(ScalarKind::Unit),
                &Shape::Ref(Box::new(Shape::Scalar(ScalarKind::Unit)))
            ),
            Unification::Match
        );
    }

    #[test]
    fn unify_ref_scalar_does_not_match_scalar() {
        // Reverse direction: &str (Ref(Scalar)) returned where String
        // (Scalar) expected — real mismatch, requires .to_string().
        assert_eq!(
            unify_shapes(
                &Shape::Ref(Box::new(Shape::Scalar(ScalarKind::Unit))),
                &Shape::Scalar(ScalarKind::Unit)
            ),
            Unification::Mismatch { unhandled: vec![] }
        );
    }

    // --- analyzer end to end over a hand-built CIR graph ---

    use vampiro_cir::{
        CirEdge, CirGraph, CirNode, EffectChannel, EffectResolution, Provenance, SourceSpan,
        StableId, Totality, UnwrapEvidence, UnwrapKind,
    };

    fn node(id: &str, domain: Shape, codomain: Shape) -> CirNode {
        node_in(id, domain, codomain, "src/lib.rs")
    }

    fn node_in(id: &str, domain: Shape, codomain: Shape, file: &str) -> CirNode {
        CirNode {
            id: StableId::new(id),
            domain,
            codomain,
            effect: EffectChannel::Plain,
            span: SourceSpan {
                file: file.into(),
                start_line: 1,
                start_column: 1,
                end_line: 1,
                end_column: 10,
            },
            name: Some(id.into()),
            trust_provenance: Default::default(),
            is_test: false,
            kind: NodeKind::Declaration,
            containing_function: None,
        }
    }

    fn edge(id: &str, source: &str, target: &str, line: usize) -> CirEdge {
        edge_with_slot(id, source, target, line, None)
    }

    fn edge_with_slot(
        id: &str,
        source: &str,
        target: &str,
        line: usize,
        slot: Option<u32>,
    ) -> CirEdge {
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
            trust_provenance: Default::default(),
            slot,
            arg_shape: None,
            return_position: true,
        }
    }

    // --- vampiro-s3e: unit callee codomain gated on same-language Rust ---

    #[test]
    fn analyze_unit_callee_codomain_gated_on_rust_boundary() {
        let mut graph = CirGraph::new("src/lib.rs");
        // Caller returns String; callee is a genuine unit-returning function.
        // rustc proves unit-vs-value return breaks on compiling Rust code, so
        // this edge must not fire a composition finding.
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::String),
        ));
        graph.add_node(node_in(
            "callee",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
            "src/other.rs",
        ));
        graph.add_edge(edge("e1", "caller", "callee", 7));

        let findings = CompositionAnalyzer::new().analyze(&graph);
        assert_eq!(
            findings.len(),
            0,
            "unit callee codomain must not fire on same-language Rust boundary"
        );
    }

    #[test]
    fn analyze_unit_callee_codomain_not_gated_cross_language() {
        let mut graph = CirGraph::new("src/lib.rs");
        // Cross-language edge (Rust caller → non-Rust callee): rustc provides
        // no protection here, so the gate must NOT apply and the mismatch
        // still fires.
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::String),
        ));
        graph.add_node(node_in(
            "callee",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
            "src/ffi.py",
        ));
        graph.add_edge(edge("e1", "caller", "callee", 7));

        let findings = CompositionAnalyzer::new().analyze(&graph);
        assert_eq!(
            findings.len(),
            1,
            "cross-language unit-callee mismatch must still fire"
        );
    }

    // --- vampiro-224.9: return-boundary check fires only for return-position calls ---

    #[test]
    fn analyze_return_boundary_gated_for_non_return_position_call() {
        let mut graph = CirGraph::new("src/lib.rs");
        // Caller returns String, callee produces Int — a codomain mismatch,
        // but the call is statement-position: the callee's result never
        // flows to the caller's return, so the return-boundary check must
        // not fire (definitionally an FP on compiling code).
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::String),
        ));
        graph.add_node(node(
            "callee",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Int),
        ));
        let mut e = edge("e1", "caller", "callee", 7);
        e.return_position = false;
        graph.add_edge(e);

        let findings = CompositionAnalyzer::new().analyze(&graph);
        assert_eq!(
            findings.len(),
            0,
            "non-return-position call must not fire the return-boundary check"
        );
    }

    #[test]
    fn analyze_return_boundary_fires_for_return_position_call() {
        let mut graph = CirGraph::new("src/lib.rs");
        // Same shapes as above, but the call is in return position (tail
        // expression / return operand) — the codomain mismatch fires.
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::String),
        ));
        graph.add_node(node(
            "callee",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Int),
        ));
        let mut e = edge("e1", "caller", "callee", 7);
        e.return_position = true;
        graph.add_edge(e);

        let findings = CompositionAnalyzer::new().analyze(&graph);
        assert_eq!(
            findings.len(),
            1,
            "return-position codomain mismatch must fire"
        );
    }

    #[test]
    fn analyze_emits_finding_on_mismatch_with_side_by_side_evidence() {
        let mut graph = CirGraph::new("src/lib.rs");
        // caller returns Record (non-void), callee produces Union[Scalar,Opaque]
        // → codomain mismatch (Record ≠ Union)
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Record(vec![
                Shape::Scalar(ScalarKind::Unit),
                Shape::Scalar(ScalarKind::Unit),
            ]),
        ));
        graph.add_node(node(
            "callee",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Union(vec![Shape::Scalar(ScalarKind::Unit), Shape::Opaque]),
        ));
        graph.add_edge(edge("e1", "caller", "callee", 7));

        let findings = CompositionAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 1, "exactly one composition finding");
        let f = &findings[0];
        assert_eq!(f.rule, "REQ-7");
        assert_eq!(f.axis, crate::finding::Axis::Composition);
        assert_eq!(f.severity, crate::finding::Severity::Medium);
        assert_eq!(f.line_range, crate::finding::LineRange::new(7, 7));

        // Now produces Union[Scalar|Opaque], caller returns Record[Scalar,Scalar].
        // Neither union arm matches the Record → both arms are unhandled.
        #[allow(irrefutable_let_patterns)]
        let crate::finding::Evidence::CompositionMismatch {
            caller_expected,
            callee_produced,
            unhandled,
        } = &f.evidence
        else {
            panic!("expected composition mismatch evidence");
        };
        assert_eq!(
            caller_expected,
            &Shape::Record(vec![
                Shape::Scalar(ScalarKind::Unit),
                Shape::Scalar(ScalarKind::Unit)
            ])
        );
        assert_eq!(
            callee_produced,
            &Shape::Union(vec![Shape::Opaque, Shape::Scalar(ScalarKind::Unit)])
        );
        assert_eq!(
            unhandled,
            &vec![Shape::Opaque, Shape::Scalar(ScalarKind::Unit)]
        );
    }

    #[test]
    fn analyze_no_finding_on_match() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ));
        graph.add_node(node(
            "callee",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ));
        graph.add_edge(edge("e1", "caller", "callee", 7));
        assert!(CompositionAnalyzer::new().analyze(&graph).is_empty());
    }

    #[test]
    fn analyze_opaque_shape_excluded() {
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ));
        // Callee codomain opaque → excluded by REQ-23.
        graph.add_node(node(
            "callee",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Opaque,
        ));
        graph.add_edge(edge("e1", "caller", "callee", 7));
        assert!(
            CompositionAnalyzer::new().analyze(&graph).is_empty(),
            "opaque codomain must not raise a composition finding"
        );
    }

    #[test]
    fn analyze_non_void_match_is_silent() {
        // Caller returns Record, callee returns same Record → match → no finding.
        let mut graph = CirGraph::new("src/lib.rs");
        let rec = Shape::Record(vec![
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ]);
        graph.add_node(node("caller", Shape::Scalar(ScalarKind::Unit), rec.clone()));
        graph.add_node(node("callee", Shape::Scalar(ScalarKind::Unit), rec));
        graph.add_edge(edge("e1", "caller", "callee", 7));
        assert!(CompositionAnalyzer::new().analyze(&graph).is_empty());
    }

    // --- vampiro-51v: slot-boundary check ---

    #[test]
    fn slot_match_produces_no_finding() {
        // Callee domain = Record[Scalar, Scalar] (a 2-param function where
        // each param is Scalar). Slot 0 expects Scalar. Caller returns Scalar
        // and passes it at slot 0 → match → no findings.
        // Caller's codomain is Scalar (void), so return-boundary check skips.
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ));
        graph.add_node(node(
            "callee",
            Shape::Record(vec![
                Shape::Scalar(ScalarKind::Unit),
                Shape::Scalar(ScalarKind::Unit),
            ]),
            Shape::Scalar(ScalarKind::Unit),
        ));
        graph.add_edge(edge_with_slot("e1", "caller", "callee", 7, Some(0)));
        assert!(CompositionAnalyzer::new().analyze(&graph).is_empty());
        assert!(CompositionAnalyzer::new().analyze(&graph).is_empty());
    }

    #[test]
    fn slot_mismatch_emits_finding() {
        // Callee domain = Record[Scalar, Scalar] (2-param function).
        // Slot 0 expects Scalar. Caller returns Record[Scalar,Scalar] and
        // passes it at slot 0, but the slot expects Scalar → mismatch.
        // Both codomains match (both return Scalar) so return-boundary
        // produces no finding.
        let mut graph = CirGraph::new("src/lib.rs");
        let rec = Shape::Record(vec![
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ]);
        // caller returns Record (non-void), domain is irrelevant
        graph.add_node(node("caller", Shape::Scalar(ScalarKind::Unit), rec.clone()));
        // callee has 2 Scalar params (Record domain), returns Scalar
        graph.add_node(node(
            "callee",
            Shape::Record(vec![
                Shape::Scalar(ScalarKind::Unit),
                Shape::Scalar(ScalarKind::Unit),
            ]),
            rec.clone(), // same codomain as caller → return-boundary match
        ));
        graph.add_edge(CirEdge {
            id: StableId::new("e1"),
            source: StableId::new("caller"),
            target: StableId::new("callee"),
            resolution: EffectResolution::Propagated,
            unwrap_evidence: None,
            provenance: Provenance::Direct,
            span: SourceSpan {
                file: "src/lib.rs".into(),
                start_line: 7,
                start_column: 5,
                end_line: 7,
                end_column: 20,
            },
            discard_spans: Vec::new(),
            trust_provenance: Default::default(),
            slot: Some(0),
            arg_shape: Some(rec.clone()), // passes Record where Scalar expected
            return_position: true,
        });
        let findings = CompositionAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 1, "expected 1 SlotMismatch finding");
        let f = &findings[0];
        assert_eq!(f.rule, "REQ-7");
        assert_eq!(f.classification, "composition-break");
        #[allow(irrefutable_let_patterns)]
        let crate::finding::Evidence::SlotMismatch {
            slot,
            callee_expected,
            caller_produced,
        } = &f.evidence
        else {
            panic!("expected SlotMismatch evidence, got: {:?}", f.evidence);
        };
        assert_eq!(*slot, 0);
        assert_eq!(*callee_expected, Shape::Scalar(ScalarKind::Unit));
        assert_eq!(*caller_produced, rec);
    }

    #[test]
    fn slot_mismatch_multi_param_callee() {
        // Callee domain = Record[Record[Scalar,Scalar], Scalar].
        // Caller returns Scalar, callee returns Scalar (return-boundary match).
        // Caller passes Scalar at slot 0, but callee expects Record at
        // that slot → slot mismatch finding.
        let mut graph = CirGraph::new("src/lib.rs");
        let inner_rec = Shape::Record(vec![
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ]);
        let callee_domain = Shape::Record(vec![inner_rec.clone(), Shape::Scalar(ScalarKind::Unit)]);
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ));
        graph.add_node(node(
            "callee",
            callee_domain,
            Shape::Scalar(ScalarKind::Unit),
        ));
        graph.add_edge(CirEdge {
            id: StableId::new("e1"),
            source: StableId::new("caller"),
            target: StableId::new("callee"),
            resolution: EffectResolution::Propagated,
            unwrap_evidence: None,
            provenance: Provenance::Direct,
            span: SourceSpan {
                file: "src/lib.rs".into(),
                start_line: 7,
                start_column: 5,
                end_line: 7,
                end_column: 20,
            },
            discard_spans: Vec::new(),
            trust_provenance: Default::default(),
            slot: Some(0),
            arg_shape: Some(Shape::Scalar(ScalarKind::Unit)), // passes Scalar where Record expected
            return_position: true,
        });
        let findings = CompositionAnalyzer::new().analyze(&graph);
        assert_eq!(findings.len(), 1);
        #[allow(irrefutable_let_patterns)]
        let crate::finding::Evidence::SlotMismatch { .. } = &findings[0].evidence
        else {
            panic!("expected SlotMismatch evidence for slot 0");
        };
    }

    #[test]
    fn slot_mismatch_slot_1_passes() {
        // Callee domain = Record[Scalar, Record[Scalar,Scalar]].
        // Caller returns Record[Scalar,Scalar], callee codomain = same Record
        // (return-boundary match). Caller passes Record at slot 1 → match.
        let mut graph = CirGraph::new("src/lib.rs");
        let inner_rec = Shape::Record(vec![
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ]);
        let callee_domain = Shape::Record(vec![Shape::Scalar(ScalarKind::Unit), inner_rec.clone()]);
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            inner_rec.clone(),
        ));
        graph.add_node(node("callee", callee_domain, inner_rec));
        graph.add_edge(edge_with_slot("e1", "caller", "callee", 7, Some(1)));
        assert!(CompositionAnalyzer::new().analyze(&graph).is_empty());
    }

    #[test]
    fn slot_no_slot_edge_skips_slot_check() {
        // Edge without a slot should not produce a slot-boundary finding.
        // The return-boundary check still runs.
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ));
        graph.add_node(node(
            "callee",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ));
        graph.add_edge(edge("e1", "caller", "callee", 7));
        assert!(CompositionAnalyzer::new().analyze(&graph).is_empty());
    }

    // --- vampiro-0j8: ? operator unwrapping ---

    #[test]
    fn try_operator_result_unwrap_no_finding() {
        // Caller returns String, callee returns Result<String, Error>.
        // Edge has Ordinary+Total unwrap evidence (? operator) → compare
        // unwrapped type (String) instead of the full Result.
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Parameterized {
                base: "String".into(),
                parameters: vec![Shape::Scalar(ScalarKind::Unit)],
            },
        ));
        graph.add_node(node(
            "callee",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Parameterized {
                base: "Result".into(),
                parameters: vec![
                    Shape::Parameterized {
                        base: "String".into(),
                        parameters: vec![Shape::Scalar(ScalarKind::Unit)],
                    },
                    Shape::Parameterized {
                        base: "Error".into(),
                        parameters: vec![Shape::Scalar(ScalarKind::Unit)],
                    },
                ],
            },
        ));
        graph.add_edge(CirEdge {
            id: StableId::new("e1"),
            source: StableId::new("caller"),
            target: StableId::new("callee"),
            resolution: EffectResolution::Unwrapped,
            unwrap_evidence: Some(UnwrapEvidence {
                kind: UnwrapKind::Ordinary,
                totality: Totality::Total,
            }),
            provenance: Provenance::Direct,
            span: SourceSpan {
                file: "src/lib.rs".into(),
                start_line: 3,
                start_column: 5,
                end_line: 3,
                end_column: 20,
            },
            discard_spans: Vec::new(),
            trust_provenance: Default::default(),
            slot: None,
            arg_shape: None,
            return_position: true,
        });

        let findings = CompositionAnalyzer::new().analyze(&graph);
        assert!(
            findings.is_empty(),
            "? operator unwraps Result -> inner value, no mismatch"
        );
    }

    #[test]
    fn try_operator_option_unwrap_no_finding() {
        // Caller returns f64, callee returns Option<f64>.
        // Edge has Ordinary+Total unwrap evidence → compare unwrapped type.
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Scalar(ScalarKind::Unit),
        ));
        graph.add_node(node(
            "callee",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Parameterized {
                base: "Option".into(),
                parameters: vec![Shape::Scalar(ScalarKind::Unit)],
            },
        ));
        graph.add_edge(CirEdge {
            id: StableId::new("e1"),
            source: StableId::new("caller"),
            target: StableId::new("callee"),
            resolution: EffectResolution::Unwrapped,
            unwrap_evidence: Some(UnwrapEvidence {
                kind: UnwrapKind::Ordinary,
                totality: Totality::Total,
            }),
            provenance: Provenance::Direct,
            span: SourceSpan {
                file: "src/lib.rs".into(),
                start_line: 3,
                start_column: 5,
                end_line: 3,
                end_column: 20,
            },
            discard_spans: Vec::new(),
            trust_provenance: Default::default(),
            slot: None,
            arg_shape: None,
            return_position: true,
        });

        let findings = CompositionAnalyzer::new().analyze(&graph);
        assert!(
            findings.is_empty(),
            "? operator unwraps Option -> inner value, no mismatch"
        );
    }

    #[test]
    fn try_operator_real_mismatch_still_fires() {
        // Caller returns String, callee returns Result<i32, Error>.
        // Edge has Ordinary+Total unwrap → compare i32 vs String → mismatch.
        let mut graph = CirGraph::new("src/lib.rs");
        graph.add_node(node(
            "caller",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Parameterized {
                base: "String".into(),
                parameters: vec![Shape::Scalar(ScalarKind::Unit)],
            },
        ));
        graph.add_node(node(
            "callee",
            Shape::Scalar(ScalarKind::Unit),
            Shape::Parameterized {
                base: "Result".into(),
                parameters: vec![
                    Shape::Scalar(ScalarKind::Unit),
                    Shape::Parameterized {
                        base: "Error".into(),
                        parameters: vec![Shape::Scalar(ScalarKind::Unit)],
                    },
                ],
            },
        ));
        graph.add_edge(CirEdge {
            id: StableId::new("e1"),
            source: StableId::new("caller"),
            target: StableId::new("callee"),
            resolution: EffectResolution::Unwrapped,
            unwrap_evidence: Some(UnwrapEvidence {
                kind: UnwrapKind::Ordinary,
                totality: Totality::Total,
            }),
            provenance: Provenance::Direct,
            span: SourceSpan {
                file: "src/lib.rs".into(),
                start_line: 3,
                start_column: 5,
                end_line: 3,
                end_column: 20,
            },
            discard_spans: Vec::new(),
            trust_provenance: Default::default(),
            slot: None,
            arg_shape: None,
            return_position: true,
        });

        let findings = CompositionAnalyzer::new().analyze(&graph);
        assert_eq!(
            findings.len(),
            1,
            "i32 != String should still produce composition finding"
        );
    }

    // --- vampiro-af2.5: fine-grained scalar kinds ---

    #[test]
    fn unify_int_does_not_match_float() {
        assert_eq!(
            unify_shapes(
                &Shape::Scalar(ScalarKind::Int),
                &Shape::Scalar(ScalarKind::Float)
            ),
            Unification::Mismatch { unhandled: vec![] }
        );
    }

    #[test]
    fn unify_same_scalar_kind_matches() {
        assert_eq!(
            unify_shapes(
                &Shape::Scalar(ScalarKind::Int),
                &Shape::Scalar(ScalarKind::Int)
            ),
            Unification::Match
        );
        assert_eq!(
            unify_shapes(
                &Shape::Scalar(ScalarKind::String),
                &Shape::Scalar(ScalarKind::String)
            ),
            Unification::Match
        );
    }

    #[test]
    fn unify_string_matches_ref_string() {
        // String literal (Scalar(String)) passed where &str (Ref(Scalar(String)))
        // expected — Rust auto-refs.
        assert_eq!(
            unify_shapes(
                &Shape::Scalar(ScalarKind::String),
                &Shape::Ref(Box::new(Shape::Scalar(ScalarKind::String)))
            ),
            Unification::Match
        );
    }

    #[test]
    fn unify_string_does_not_match_ref_int() {
        // String passed where &i32 expected — should NOT match.
        assert_eq!(
            unify_shapes(
                &Shape::Scalar(ScalarKind::String),
                &Shape::Ref(Box::new(Shape::Scalar(ScalarKind::Int)))
            ),
            Unification::Mismatch { unhandled: vec![] }
        );
    }

    #[test]
    fn unify_vec_matches_slice() {
        // Vec[T] ↔ slice[T] parameterized base aliasing.
        let vec_int = Shape::Parameterized {
            base: "Vec".into(),
            parameters: vec![Shape::Scalar(ScalarKind::Int)],
        };
        let slice_int = Shape::Parameterized {
            base: "slice".into(),
            parameters: vec![Shape::Scalar(ScalarKind::Int)],
        };
        assert_eq!(
            unify_shapes(&vec_int, &slice_int),
            Unification::Match,
            "Vec[T] should match slice[T]"
        );
        assert_eq!(
            unify_shapes(&slice_int, &vec_int),
            Unification::Match,
            "slice[T] should match Vec[T]"
        );
    }

    #[test]
    fn unify_vec_does_not_match_different_param() {
        // Vec[Int] should NOT match slice[String].
        let vec_int = Shape::Parameterized {
            base: "Vec".into(),
            parameters: vec![Shape::Scalar(ScalarKind::Int)],
        };
        let slice_string = Shape::Parameterized {
            base: "slice".into(),
            parameters: vec![Shape::Scalar(ScalarKind::String)],
        };
        assert_eq!(
            unify_shapes(&vec_int, &slice_string),
            Unification::Mismatch { unhandled: vec![] }
        );
    }
}
