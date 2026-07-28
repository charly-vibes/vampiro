# Verification: Section 1 — Composition Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.4.2
**Spec:** `openspec/changes/add-core-seam-analysis/specs/seam-analysis/spec.md`
**Canonicalization decision:** `docs/decisions/shape-canonicalization.md` (vampiro-0vb.4.1, approved 2026-07-28)

## Summary

Delivered the composition tracer (REQ-7, REQ-23) as the first slice of the
`vampiro-seam-analysis` crate. Introduces the normalized finding contract
(REQ-4, EARS v1.3.0): the closed axis set `{composition, modularity,
optionality, robustness}`, the `LOW`/`MEDIUM`/`HIGH` severity vocabulary,
per-rule default severities (composition break = `MEDIUM`), and the
side-by-side `CompositionMismatch` evidence payload. Shape canonicalization
(normalize + 128-bit canonical hash) is implemented in `vampiro-cir` per the
approved decision.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 1.1 | ✓ | 12 failing-then-passing tests for structural unification, union-arm handling, opaque-shape exclusion (REQ-23), and side-by-side mismatch evidence (REQ-7). |
| 1.2 | ✓ | Coarse structural normalization/unification under the approved canonicalization contract; opaque edges preserved for non-composition checks. |
| 1.3 | ✓ | 15 focused unit tests + 1 Rust frontend E2E negative fixture; expected finding fields and command output recorded below. |

## Implementation

### Shape canonicalization — `crates/vampiro-cir/src/shape.rs`

| Feature | Implementation |
|---------|---------------|
| `Shape::normalize` | Pure structural canonicalization: union arms and record fields sorted by canonical JSON serialization (unordered-set semantics); `Parameterized` parameter order preserved (positional sums); `Function`/`Ref` normalize recursively; `Opaque`/`Bottom` preserved as leaves. Idempotent. |
| `Shape::to_canonical_json` | Compact, deterministic JSON of the normalized shape (REQ-29 byte-reproducibility form). |
| `Shape::canonical_hash` | `SHA256(canonical_json(normalize(self)))` truncated to 128 bits, hex-encoded — the same scheme as `StableId`. The shape-hash component of the REQ-24 dedupe identity. |

### Composition tracer — `crates/vampiro-seam-analysis/`

| Module | Contents |
|--------|----------|
| `src/finding.rs` | `Axis` (closed 4-value set), `Severity` (`low`/`medium`/`high`), `LineRange`, `Evidence` (currently `CompositionMismatch`), `Finding` with `composition_mismatch` builder (default severity `MEDIUM` per REQ-4 table). |
| `src/composition.rs` | `unify_shapes(produced, expected) -> Unification` (`Match` / `Mismatch{unhandled}` / `OpaqueExcluded`) and `CompositionAnalyzer::analyze(&CirGraph) -> Vec<Finding>`. |
| `src/lib.rs` | `analyze(&CirGraph) -> Vec<Finding>` entry point dispatching to the implemented slices (composition only today). |

### Unification semantics (coarse, per EARS §1)

- Top-level `Opaque` produced or expected → `OpaqueExcluded` (REQ-23); no composition finding; the edge remains eligible for modularity/robustness checks.
- Normalized-equal shapes → `Match`.
- Produced `Union`, expected non-union → `Mismatch` with the arms the caller left unhandled (the `parse_amount` worked example: `union<Decimal,None>` expected `Decimal` → unhandled `[None]`).
- Expected `Union`, produced non-union → `Match` if the produced value is covered by some arm (the caller accepts a sum; the other arms simply do not occur here), else `Mismatch`.
- Cross-variant / leaf mismatch → `Mismatch` with empty `unhandled`.

## Fixtures

Located at `tests/fixtures/add-core-seam-analysis/1/`:

| Fixture | Rust source | Nodes | Edges | Finding |
|---------|-------------|-------|-------|---------|
| `composition_break.rs` | `parse_amount -> Option<f64>` fed into `apply_discount(amount: f64, …)` | 3 | 2 | ≥1 `REQ-7` composition finding |

## Expected finding fields

A composition finding produced by the tracer carries (REQ-4, REQ-7):

| Field | Value |
|-------|-------|
| `rule` | `REQ-7` |
| `axis` | `composition` |
| `severity` | `medium` (default; REQ-4 table) |
| `line-range-start` / `line-range-end` | edge span |
| `evidence.caller-expected` | caller domain shape (normalized) |
| `evidence.callee-produced` | callee codomain shape (normalized) |
| `evidence.unhandled` | unhandled union arms (normalized), empty for cross-variant mismatches |
| `filtration-distance` | absent (no filtration declared; computed at REQ-C2 when one is) |

## Passing command output

```
$ cargo test -p vampiro-cir --lib shape::
test result: ok. 31 passed; 0 failed; 0 ignored; 0 measured; 57 filtered out

$ cargo test -p vampiro-seam-analysis
test result: ok. 15 passed; 0 failed; 0 ignored (lib)
test result: ok.  1 passed; 0 failed; 0 ignored (composition_e2e)

$ cargo test --workspace
total passed: 274   (all crates, 0 failed)

$ cargo fmt --all --check
(clean)

$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile in 1.25s   (no warnings)

$ openspec validate add-core-seam-analysis --strict
Change 'add-core-seam-analysis' is valid
```

## Contract versions

| Contract | Version | Location |
|----------|---------|----------|
| CIR schema | `0.1.0` | `vampiro-cir` crate; `CirGraph.version` |
| Shape canonicalization | internal to CIR `0.1.0` (decision §4) | `Shape::normalize` / `Shape::canonical_hash` |
| Normalized finding contract | `0.1.0` (in-progress; formally published at `0vb.4.6`) | `vampiro-seam-analysis::finding` |

## Known limitation / refinement

The composition tracer operates over CIR **call edges** (frontend `source`=caller,
`target`=callee), which approximate the EARS data-flow edge ("an argument at
the call site derives … from the callee's return value"). Without per-slot
argument-to-parameter binding on the edge, the coarse check compares the
callee's codomain against the caller's whole domain. The `unify_shapes`
primitive is correct; the wiring is the approximation. Per-slot argument
binding is tracked as a refinement (follow-up ticket `vampiro-0vb.4.7`).

## Owned requirement traceability

| Requirement | Test(s) |
|-------------|---------|
| REQ-7 (composition break) | `unify_union_subset_unhandled`, `analyze_emits_finding_on_mismatch_with_side_by_side_evidence`, `composition_e2e_negative_fixture` |
| REQ-23 (opaque exclusion) | `unify_produced_opaque_excluded`, `unify_expected_opaque_excluded`, `analyze_opaque_shape_excluded` |
| REQ-4 (finding schema / default severities) | `axis_serializes_to_closed_kebab_set`, `severity_serializes_to_lowercase`, `composition_finding_default_severity_is_medium` |
| Shape canonicalization decision | `normalize_*`, `canonical_hash_*`, `canonical_json_round_trips` (31 tests in `vampiro-cir`) |
