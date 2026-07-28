# Verification: Section 4 — Platform Acceptance

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.2.5
**Spec:** `openspec/changes/add-cir-plugin-platform/specs/cir-plugin-platform/spec.md`

## Summary

Accepted and published the CIR plugin platform contracts. All quality gates pass, requirement traceability verified, consumer compatibility confirmed.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 4.1 | ✓ | Ran all declaration, model, frontend trait, and integration suites. Full workspace test, format, and Clippy all pass. |
| 4.2 | ✓ | Requirement traceability verified. Consumer compatibility test imports `vampiro-cir`, constructs `CirGraph`, calls `validate()`, implements `Frontend`, and integrates with CLI finding/config contracts. |
| 4.3 | ✓ | `openspec validate add-cir-plugin-platform --strict` passes. CIR schema, plugin boundary, fixture versions, and commands recorded below. |

## Quality Gates

| Gate | Status | Output |
|------|--------|--------|
| `cargo test --workspace` | ✓ | 98 tests pass (67 unit + 14 frontend conformance + 9 consumer + 4 category + 4 round-trip) |
| `cargo fmt --check` | ✓ | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✓ | Clean |
| `openspec validate add-cir-plugin-platform --strict` | ✓ | "Change 'add-cir-plugin-platform' is valid" |

## CIR Consumer Contract

| Contract | Value | Evidence |
|----------|-------|----------|
| CIR schema version | `0.1.0` | `CirGraph::version` field, serialized as `"version": "0.1.0"` |
| Plugin boundary | Workspace-crate ABI (in-process trait dispatch) | `docs/decisions/plugin-boundary.md` |
| Frontend trait | `language() -> &'static str`, `extract(source, path) -> Result<CirGraph, CirError>` | `crates/vampiro-cir/src/frontend.rs` |
| Max effect depth | 64 | `crates/vampiro-cir/src/effect.rs:MAX_EFFECT_DEPTH` |
| Max shape depth | 64 | `crates/vampiro-cir/src/shape.rs:MAX_SHAPE_DEPTH` |
| Max closure size | 4096 | `crates/vampiro-cir/src/category.rs:MAX_CLOSURE_SIZE` |
| Max filtration levels | 16 | `crates/vampiro-cir/src/category.rs:MAX_FILTRATION_LEVELS` |

## Fixture Paths

| Section | Path | Tests |
|---------|------|-------|
| 1. CIR round-trip | `tests/fixtures/add-cir-plugin-platform/1/` | 4 fixture integration tests |
| 2. Category/filtration | `tests/fixtures/add-cir-plugin-platform/2/` | 4 fixture integration tests |
| 3. Frontend conformance | `tests/fixtures/add-cir-plugin-platform/3/` | 14 conformance tests |
| Consumer | `crates/vampiro-cli/tests/cir_consumer_tests.rs` | 9 consumer compatibility tests |

## Requirement Traceability

| Requirement | Spec | Tests |
|-------------|------|-------|
| CIR preserves compositional structure (REQ-1–3, REQ-21) | `cir-plugin-platform/spec.md` | 22 effect/shape unit tests + 4 fixture round-trips |
| Visibility extraction is a platform contract (REQ-V1, REQ-V2) | `cir-plugin-platform/spec.md` | `Frontend` trait contract (3 tests) |
| Categories and filtrations are extensible and valid (REQ-C1–C3, C8–C9) | `cir-plugin-platform/spec.md` | 30 category/filtration tests + 4 fixture tests |
| Plugins pass reproducible structural conformance (REQ-6, REQ-29, REQ-C10) | `cir-plugin-platform/spec.md` | 14 frontend conformance tests, 9 consumer tests |
| Unknown idioms and plugin conflicts fail safely (REQ-21, REQ-22) | `cir-plugin-platform/spec.md` | `Unknown` sentinel in effect/resolution, depth-limit rejection (STUB: multi-plugin scenarios deferred) |

## Commands Executed

```bash
# Full workspace test
cargo test --workspace
# → 98 tests pass

# Format check
cargo fmt --check
# → clean

# Clippy with warnings denied
cargo clippy --workspace --all-targets -- -D warnings
# → clean

# OpenSpec validation
openspec validate add-cir-plugin-platform --strict
# → "Change 'add-cir-plugin-platform' is valid"
```

## Published Contracts

- **CIR schema v0.1.0** — `vampiro-cir` crate, `CirGraph` type with JSON serialization
- **Frontend trait** — `vampiro_cir::frontend::Frontend` (in-process trait dispatch)
- **Category/filtration types** — `vampiro_cir::category::{CategoryDecl, FiltrationDecl, ValidatedCategory}`
- **Plugin boundary decision** — `docs/decisions/plugin-boundary.md` (workspace-crate ABI, no serialization)
- **Consumer test** — `crates/vampiro-cli/tests/cir_consumer_tests.rs` (demonstrates end-to-end integration)