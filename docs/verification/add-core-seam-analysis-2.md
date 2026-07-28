# Verification: Section 2 — Modularity Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.4.3
**Spec:** `openspec/changes/add-core-seam-analysis/specs/seam-analysis/spec.md`

## Summary

Delivered the modularity tracer (REQ-8, REQ-V3–V4, REQ-V7, REQ-C5) as the
second slice of `vampiro-seam-analysis`. Introduces the language-neutral
visibility lattice model (Addendum V: L0–L4 + boundary kinds), the
nesting/facade reachability check (REQ-C5), and three modularity checks:
edge-level reach-through (REQ-8/V3), declaration-level over-exposure
(REQ-V4), and facade-level facade-leak (REQ-V7). Enforced boundary crossings
produce axis-less `boundary:enforced-unreachable` diagnostics (REQ-V3),
distinct from modularity findings.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 2.1 | ✓ | 7 failing-then-passing unit tests for advisory/enforced crossings, arbitrary-depth nesting, over-exposure, facade-leak, and plugin diagnostics. |
| 2.2 | ✓ | Language-neutral visibility/facade reachability (REQ-C5), exactly-one-axis modularity findings, enforced-unreachable diagnostics outside findings. |
| 2.3 | ✓ | 16 focused unit tests + 1 Rust facade E2E test asserting over-exposure + facade-leak findings. |

## Implementation

### Language-neutral visibility model — `src/visibility.rs`

| Type | Purpose |
|------|---------|
| `LatticeLevel` (L0–L4 + L1Half) | The visibility lattice (Addendum V, ordered). |
| `BoundaryKind` (Enforced/Advisory/EnforcedOpen) | How a level is enforced. |
| `VisibilityFact` | Per-node `(level, boundary, scope, internal_by_convention)`. |
| `VisibilityFacts` | The facts table + nesting edges + facade re-exports. Provides `nesting_reachable` and `facade_reachable` (REQ-C5 generators). |

### Modularity tracer — `src/modularity.rs`

| Check | Rule | Classification | Output |
|-------|------|----------------|--------|
| Edge-level reach-through | REQ-8, REQ-V3, REQ-C5 | `reach-through` | `Finding` (axis=modularity) or `Diagnostic` (enforced-unreachable) |
| Declaration-level over-exposure | REQ-V4 | `over-exposure` | `Finding` (axis=modularity) |
| Facade-level facade-leak | REQ-V7 | `facade-leak` | `Finding` (axis=modularity) |

### Finding contract additions — `src/finding.rs`

| Addition | Details |
|----------|---------|
| `Evidence::ReachThrough` | target_level, target_boundary, boundary_crossed |
| `Evidence::OverExposure` | target_level, convention |
| `Evidence::FacadeLeak` | facade_scope, exported_name, underlying_level |
| `Finding.classification` | New field (e.g. `reach-through`, `over-exposure`, `facade-leak`) |
| `Diagnostic` | Axis-less, no severity — for `boundary:enforced-unreachable` |

### Reachability (REQ-C5, REQ-C3)

The legitimate subcategory 𝒢 is built from:
- **Nesting generators**: child→parent scope edges. `nesting_reachable(caller, target)` walks the caller's ancestry; if `target` is an ancestor of or equal to `caller`, the caller is inside the target's scope → legitimate.
- **Facade/export generators**: re-export entries. `facade_reachable(caller, target_node)` checks if the target is re-exported in a facade reachable from the caller via nesting.
- Arbitrary depth (REQ-C3): the walk has no fixed limit; cycle-guarded.

## Fixtures

Located at `tests/fixtures/add-core-seam-analysis/2/`:

| Fixture | Rust source | Nodes | Findings |
|---------|-------------|-------|----------|
| `modularity_break.rs` | `pub fn _helper` (L3, doc-hidden convention) + `pub use internal::raw_helper` (L2 re-exported at L4) | 3 | ≥1 over-exposure + ≥1 facade-leak |

## Finding/diagnostic schemas

**Over-exposure finding (REQ-V4)**:
```json
{"rule":"REQ-V4","axis":"modularity","severity":"medium","classification":"over-exposure",
 "evidence":{"target_level":"L3","convention":"doc(hidden) / leading-underscore / excluded from facade"}}
```

**Facade-leak finding (REQ-V7)**:
```json
{"rule":"REQ-V7","axis":"modularity","severity":"medium","classification":"facade-leak",
 "evidence":{"facade_scope":"...","exported_name":"raw_helper","underlying_level":"L2"}}
```

**Enforced-unreachable diagnostic (REQ-V3)**:
```json
{"diagnostic":"boundary:enforced-unreachable","detail":"edge to ... crosses an enforced boundary; ..."}
```
(No `axis`, no `severity` — diagnostics are not findings.)

## Passing command output

```
$ cargo test -p vampiro-seam-analysis --lib -- visibility:: modularity::
test result: ok. 16 passed; 0 failed; 0 ignored

$ cargo test -p vampiro-seam-analysis
test result: ok. 31 passed; 0 failed (lib)
test result: ok.  1 passed; 0 failed (composition_e2e)
test result: ok.  1 passed; 0 failed (modularity_e2e)

$ cargo test --workspace
total passed: 291   (all crates, 0 failed)

$ cargo fmt --all --check
(clean)

$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile in 0.41s   (no warnings)

$ openspec validate add-core-seam-analysis --strict
Change 'add-core-seam-analysis' is valid
```

## Owned requirement traceability

| Requirement | Test(s) |
|-------------|---------|
| REQ-8 (reach-through) | `advisory_crossing_raises_reach_through_finding` |
| REQ-V3 (enforced-unreachable diagnostic) | `enforced_crossing_raises_diagnostic_not_finding` |
| REQ-V4 (over-exposure) | `over_exposure_for_doc_hidden_pub`, `no_over_exposure_for_facade_item`, `modularity_e2e_over_exposure_and_facade_leak` |
| REQ-V7 (facade-leak) | `facade_leak_for_deep_underlying_level`, `no_facade_leak_when_underlying_is_l4`, `modularity_e2e_over_exposure_and_facade_leak` |
| REQ-C5 (scope-category reachability) | `nesting_reachable_no_finding`, `facade_reachable_no_finding`, `nesting_reachable_arbitrary_depth`, `nesting_reachable_ancestor` |
| REQ-C3 (arbitrary filtration depth) | `nesting_reachable_arbitrary_depth` |
| REQ-4 (exactly-one-axis) | `all_modularity_findings_use_modularity_axis` |
