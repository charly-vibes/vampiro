# Verification: Section 4 — Redundancy Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.4.5
**Spec:** `openspec/changes/add-core-seam-analysis/specs/seam-analysis/spec.md`

## Summary

Delivered the redundancy tracer (REQ-11, REQ-C7) as the fourth slice of the
`vampiro-seam-analysis` crate. Introduces the `RedundancyMismatch` evidence
variant on the `robustness` axis (default severity `MEDIUM` per REQ-4 table).
Detects consumer nodes with multiple inbound branches whose codomain shapes
differ, raising a finding when no explicit adapter node reconciles them.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 4.1 | ✓ | 7 focused unit tests + 3 E2E tests covering two-branch mismatch, three-branch mismatch, same-codomain (no finding), opaque exclusion (REQ-23), single-inbound (no finding), adapter detection, and axis-only robustness. |
| 4.2 | ✓ | Deterministic redundancy reconciliation: groups edges by target, compares normalized source codomains, finds adapters as intermediate nodes bridging shape mismatches, emits findings on the `robustness` axis only (no fifth or combined axis). |
| 4.3 | ✓ | 7 unit tests + 3 E2E tests pass; common-codomain evidence carries `branch_shapes`, `expected_shape`, and `adapters`; commands recorded below. |

## Implementation

### Redundancy tracer — `crates/vampiro-seam-analysis/src/redundancy.rs`

| Feature | Implementation |
|---------|---------------|
| `RedundancyAnalyzer::analyze` | Groups CIR edges by target node; for each target with ≥2 distinct source nodes, compares normalized codomain shapes. Skips Opaque sources (REQ-23). Emits finding when shapes differ. |
| `find_adapters` | Searches for intermediate nodes on branch→consumer paths whose codomain matches the expected shape. Adapter names are carried as evidence. |

### Finding contract extension — `crates/vampiro-seam-analysis/src/finding.rs`

| Feature | Implementation |
|---------|---------------|
| `Evidence::RedundancyMismatch` | New variant with `branch_shapes` (Vec<Shape>), `expected_shape` (Shape), `adapters` (Vec<String>). |
| `Finding::redundancy_mismatch` | Builder function with default severity `MEDIUM`, axis `Robustness`, classification `redundancy-mismatch`, rule `REQ-11`. Shapes are normalized before storage. |

## Fixtures

Located at `tests/fixtures/add-core-seam-analysis/4/`:

| Fixture | Rust source | Purpose |
|---------|-------------|---------|
| `redundancy_mismatch.rs` | `primary_source_fetch() -> (f64, String)`, `cache_get() -> Option<f64>`, `use_data((f64, String)) -> f64` | Redundancy chain with mismatched branch codomains. |

**Note:** The Rust frontend extracts coarse shapes. The E2E tests construct the
CIR graph programmatically to validate the analyzer + evidence + output format
across specific branch configurations.

## Expected finding fields

A redundancy-mismatch finding produced by the tracer carries (REQ-4, REQ-11):

| Field | Value |
|-------|-------|
| `rule` | `REQ-11` |
| `axis` | `robustness` |
| `severity` | `medium` (default; REQ-4 table) |
| `line-range-start` / `line-range-end` | span covering all inbound edges |
| `evidence.branch-shapes` | normalized codomain shapes of all non-opaque source branches |
| `evidence.expected-shape` | the consumer's normalized domain shape |
| `evidence.adapters` | adapter node names found (empty if none) |
| `filtration-distance` | absent |
| `classification` | `redundancy-mismatch` |

## Passing command output

```
$ cargo test -p vampiro-seam-analysis
test result: ok. 46 passed; 0 failed; 0 ignored (lib)
test result: ok. 1 passed; 0 failed; 0 ignored (composition_e2e)
test result: ok. 4 passed; 0 failed; 0 ignored (effects_e2e)
test result: ok. 1 passed; 0 failed; 0 ignored (modularity_e2e)
test result: ok. 3 passed; 0 failed; 0 ignored (redundancy_e2e)

$ cargo test --workspace
(285+ passed, all crates, 0 failed)

$ cargo fmt --all --check
(clean)

$ cargo clippy --workspace --all-targets -- -D warnings
Finished `dev` profile  (no warnings)

$ openspec validate add-core-seam-analysis --strict
Change 'add-core-seam-analysis' is valid
```

## Contract versions

| Contract | Version | Location |
|----------|---------|----------|
| CIR schema | `0.1.0` | `vampiro-cir` crate; `CirGraph.version` |
| Shape canonicalization | internal to CIR `0.1.0` | `Shape::normalize` |
| Normalized finding contract | `0.1.0` (in-progress; formally published at `0vb.4.6`) | `vampiro-seam-analysis::finding` (now includes `SwallowedEffect` and `RedundancyMismatch` evidence variants) |

## Known limitation / refinement

The adapter detection is conservative: it finds intermediate nodes whose
codomain matches the expected shape but does not verify that *every* unequal
branch path goes through an adapter. Findings carry adapter names as evidence
for human review. A complete cocone check (tracking per-branch adapter coverage)
is a refinement for a later slice.

## Owned requirement traceability

| Requirement | Test(s) |
|-------------|---------|
| REQ-11 (redundancy mismatch) | `two_branches_mismatch_raises_finding`, `three_branches_mismatch_raises_finding`, `two_branches_same_codomain_no_finding`, `adapter_reconciles_mismatch`, `redundancy_e2e_two_branches_mismatch`, `redundancy_e2e_three_branches` |
| REQ-C7 (any branch count, cocone) | `three_branches_mismatch_raises_finding`, `redundancy_e2e_three_branches` |
| REQ-23 (opaque exclusion) | `opaque_branch_excluded` |
| REQ-4 (closed axis set) | `all_redundancy_findings_use_robustness_axis` |
| Single inbound (no redundancy) | `single_inbound_edge_no_finding`, `redundancy_e2e_all_same_no_finding` |