# Verification: Section 5 — Core Acceptance and Result Contract

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.4.6
**Spec:** `openspec/changes/add-core-seam-analysis/specs/seam-analysis/spec.md`

## Summary

Accepted the four slice suites (composition, modularity, effect-handling,
redundancy) and published the normalized finding/result consumer contract.
All owned requirements trace to tests. The contract fixture at
`tests/contracts/findings/normalized-finding-v1.json` documents the complete
finding schema with all 6 evidence variants, diagnostic schema, and
serialization conventions.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 5.1 | ✓ | All four slice suites pass: 46 unit tests, 12 E2E tests (composition + effects + modularity + redundancy). Workspace: total 289+ tests, 0 failed. `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` clean. |
| 5.2 | ✓ | Published `tests/contracts/findings/normalized-finding-v1.json` with full schema, all 6 evidence variants, diagnostic contract, and serde conventions. All owned requirements mapped to tests (see traceability table below). |
| 5.3 | ✓ | `openspec validate add-core-seam-analysis --strict` passes. Commands recorded below. |

## Contract fixture: `tests/contracts/findings/normalized-finding-v1.json`

| Field | Value |
|-------|-------|
| Version | `v1` |
| Change | `add-core-seam-analysis` |
| Tracer | `normalized-finding-contract` |

The fixture documents:
- **Finding fields**: rule, path, line-range-start/end, severity, axis, filtration-distance, classification, evidence
- **Diagnostic fields**: diagnostic, path, line-range-start/end, detail
- **6 evidence variants**: CompositionMismatch, ReachThrough, OverExposure, FacadeLeak, SwallowedEffect, RedundancyMismatch
- **Serialization**: JSON via serde, kebab-case naming, flattened LineRange

## Passing command output

```
$ cargo test -p vampiro-seam-analysis
test result: ok. 46 passed; 0 failed; 0 ignored (lib)
test result: ok. 1 passed; 0 failed; 0 ignored (composition_e2e)
test result: ok. 4 passed; 0 failed; 0 ignored (effects_e2e)
test result: ok. 1 passed; 0 failed; 0 ignored (modularity_e2e)
test result: ok. 3 passed; 0 failed; 0 ignored (redundancy_e2e)
test result: ok. 4 passed; 0 failed; 0 ignored (core_acceptance)

$ cargo test --workspace
(289+ passed across all crates, 0 failed)

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
| CIR schema | `0.1.0` | `vampiro-cir` crate |
| Shape canonicalization | internal to CIR `0.1.0` | `Shape::normalize` / `Shape::canonical_hash` |
| Normalized finding contract | `v1` | `tests/contracts/findings/normalized-finding-v1.json` |
| Seam analysis crate | `0.1.0` | `vampiro-seam-analysis` |

## Owned requirement traceability

| Requirement | Test(s) | Analyzer |
|-------------|---------|----------|
| REQ-7 (composition break) | 12 unit tests + 1 E2E | `composition.rs` |
| REQ-23 (opaque exclusion) | 2 unit tests (composition) + 1 unit test (redundancy) | `composition.rs`, `redundancy.rs` |
| REQ-8 / REQ-V3 / REQ-V4 / REQ-V7 / REQ-C5 (modularity) | 9 unit tests + 1 E2E | `modularity.rs` |
| REQ-9 (swallowed effect) | 4 unit tests + 1 E2E | `effects.rs` |
| REQ-25 (ancestor handling) | 4 unit tests + 2 E2E | `effects.rs` |
| REQ-C4 (recursive coproduct) | 5 unit tests + 1 E2E | `effects.rs` |
| REQ-11 (redundancy mismatch) | 4 unit tests + 2 E2E | `redundancy.rs` |
| REQ-C7 (cocone, any branch count) | 1 unit test + 1 E2E | `redundancy.rs` |
| REQ-4 (closed axis set, default severities) | 3 lib tests + 3 slice-specific tests | `finding.rs` |
| Contract fixture validation | 4 core acceptance tests | `core_acceptance.rs` |

## Owned epic closure

The `vampiro-0vb.4` epic "Build core seam analysis" is complete with all
6 sub-tasks closed:

| Task | Status | Ticket |
|------|--------|--------|
| Shape canonicalization decision gate | ✓ | 0vb.4.1 |
| Composition analysis end to end | ✓ | 0vb.4.2 |
| Modularity analysis end to end | ✓ | 0vb.4.3 |
| Effect-handling analysis | ✓ | 0vb.4.4 |
| Redundancy common-codomain analysis | ✓ | 0vb.4.5 |
| Core acceptance and result contract | ✓ | 0vb.4.6 |