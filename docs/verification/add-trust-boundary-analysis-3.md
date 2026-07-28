# Verification: Section 3 — Validation-Duplication Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.10.4
**Spec:** `openspec/changes/add-trust-boundary-analysis/tasks.md#3`
**Decision record:** `docs/decisions/trust-boundary-contract.md`

## Summary

Delivered the CIR `ValidationObservation` type and the language-neutral validation-duplication analyzer. Detects when the same validation identity appears at multiple locations outside the recognized smart constructor.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 3.1 | ✓ | `ValidationObservation` struct with serde round-trip tests (2 tests in CIR crate), CirGraph integration test with validation_observations round-trip, 6 analyzer tests in seam-analysis. |
| 3.2 | ✓ | `ValidationObservation` in `vampiro-cir` (identity, constructor_id, refined_shape, span, origin). `ValidationDuplicationAnalyzer` in `vampiro-seam-analysis` — emits `REQ-B4` / `LOW` / `modularity` / `validation-duplication` finding per duplicate location. |
| 3.3 | ✓ | 6 focused tests: single observation (no finding), duplicate (1 finding), three duplicates (2 findings), different identities no collision, empty observations, different identity no duplicate. |

## Implementation

### CIR extension — `crates/vampiro-cir/src/provenance.rs`

| Type | Fields |
|------|--------|
| `ValidationObservation` | `identity: String`, `constructor_id: StableId`, `refined_shape: String`, `span: SourceSpan`, `origin: String` |

Added to `CirGraph.validation_observations: Vec<ValidationObservation>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` for backward compatibility.

### Validation-duplication analyzer — `crates/vampiro-seam-analysis/src/validation_duplication.rs`

| Feature | Implementation |
|---------|---------------|
| `ValidationDuplicationAnalyzer::analyze` | Groups observations by identity. If an identity appears ≥2 times, the first is treated as primary and all others emit findings. |

### Evidence contract

| Field | Value |
|-------|-------|
| `rule` | `REQ-B4` |
| `axis` | `modularity` |
| `severity` | `low` (default; REQ-4 table) |
| `classification` | `validation-duplication` |
| `evidence.identity` | Stable validation identity |
| `evidence.constructor_id` | Smart constructor stable identity |
| `evidence.refined_shape` | Refined shape name |
| `evidence.origin` | Recognition origin: `"declaration"` or `"idiom"` |

## Passing command output

```
$ cargo test -p vampiro-cir
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p vampiro-seam-analysis
test result: ok. 63 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --workspace
(all tests pass, 0 failed)

$ cargo fmt --all --check
(clean)

$ cargo clippy --workspace --all-targets -- -D warnings
(no warnings)

$ openspec validate add-trust-boundary-analysis --strict
Change 'add-trust-boundary-analysis' is valid
```