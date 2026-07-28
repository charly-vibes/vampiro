# Verification: Section 4 — Refinement-Confirmation Evidence Import

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.10.5
**Spec:** `openspec/changes/add-trust-boundary-analysis/tasks.md#4`
**Decision record:** `docs/decisions/trust-boundary-contract.md`

## Summary

Delivered the refinement-confirmation evidence import module. Implements the versioned evidence schema (`v0.1.0`), JSON deserialization, and deterministic correlation against analyzed revision, constructor identity, source hash, and shape hash. Produces `confirmed` or `unknown` with the ordered closed-reason vocabulary.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 4.1 | ✓ | 15 tests covering all 10 unknown reasons + 1 positive case. Evidence schema serde round-trip test. |
| 4.2 | ✓ | `correlate_evidence()` function implementing the ordered 10-step check: absent→malformed→unsupported-version→stale→mismatched→empty-classes→duplicate-class→incomplete→unknown-class→failing. |
| 4.3 | ✓ | 15 tests pass; evidence schema version `v0.1.0` published as `EVIDENCE_SCHEMA_VERSION` constant. |

## Implementation

### Evidence import — `crates/vampiro-seam-analysis/src/refinement_confirmation.rs`

| Type | Purpose |
|------|---------|
| `EvidencePayload` | Top-level evidence schema (schema_version, producer, analyzed_revision, constructor, boundary_classes) |
| `EvidenceProducer` | Companion tool identity (name, version) |
| `EvidenceConstructor` | Constructor identity and hashes (stable_identity, source_hash, shape_hash) |
| `BoundaryClassResult` | Single class result (id, status, optional details) |
| `RefinementConfirmation` | Correlation result (status: Confirmed/Unknown, optional reason) |
| `RefinementReason` | Ordered closed vocabulary (10 variants) |
| `correlate_evidence()` | Deterministic 10-step check against revision, constructor, and declared classes |

### Correlation order (first applicable reason wins)

| Step | Check | Reason |
|------|-------|--------|
| 1 | Evidence JSON is `None` | `absent` |
| 2 | JSON parse fails | `malformed` |
| 3 | `schema_version != "v0.1.0"` | `unsupported-version` |
| 4 | `analyzed_revision != current_revision` | `stale` |
| 5 | constructor identity/hash mismatch | `mismatched` |
| 6 | `boundary_classes` is empty | `empty-classes` |
| 7 | Duplicate class IDs in evidence | `duplicate-class` |
| 8 | Not all declared classes appear in evidence | `incomplete` |
| 9 | Evidence has undeclared class IDs | `unknown-class` |
| 10 | Any class has `status="failing"` | `failing` |
| — | All checks pass | `confirmed` |

## Passing command output

```
$ cargo test -p vampiro-seam-analysis
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --workspace
(all tests pass, 0 failed)

$ cargo fmt --all --check
(clean)

$ cargo clippy --workspace --all-targets -- -D warnings
(no warnings)

$ openspec validate add-trust-boundary-analysis --strict
Change 'add-trust-boundary-analysis' is valid
```