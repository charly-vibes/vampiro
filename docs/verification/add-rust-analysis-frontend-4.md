# Verification: Section 4 — Lifecycle Fact Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.3.5
**Spec:** `openspec/changes/add-rust-analysis-frontend/tasks.md#4`

## Summary

Delivered lifecycle fact extraction. Extracts write facts, retry facts, resource identity, exit paths, and aliases without lifecycle classification or findings. Lifecycle-fact schema v0.1.0.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 4.1 | ✓ | Added fixtures for write/retry facts, resource identity, acquisition/release, exit paths, aliases. |
| 4.2 | ✓ | Implemented lifecycle extraction hooks. No lifecycle classification or findings — extraction only. |
| 4.3 | ✓ | 13 tests pass. Lifecycle-fact schema v0.1.0 published. |

## Implementation

**Module:** `crates/vampiro-rust-frontend/src/lifecycle.rs`

| Type | Schema Version | Description |
|------|---------------|-------------|
| `LifecycleFacts` | 0.1.0 | Top-level container for all lifecycle facts |
| `WriteFact` | — | Variable/field being written: target, kind, function, span |
| `RetryFact` | — | Loop kind, function, break/continue presence, span |
| `ResourceFact` | — | Resource identity: variable, type, kind, event, span |
| `ExitPathFact` | — | Exit path: function, kind (early-return, panic), span |
| `AliasFact` | — | Alias reference: original, alias, function, span |

## Fixtures

Located at `tests/fixtures/add-rust-analysis-frontend/4/`:

| Fixture | Content |
|---------|---------|
| `lifecycle-writes.json` | Write fact with target "x", kind "assignment", function "foo" |
| `lifecycle-exits.json` | Exit path with kind "early-return", function "foo" |

## Test Suite

### Lifecycle extraction tests (13 new)
| Test | What it verifies |
|------|-----------------|
| `empty_file_produces_empty_facts` | Empty source → empty facts |
| `extract_write_assignment` | `x = 42` → WriteFact |
| `extract_loop_retry` | `loop { break; }` → RetryFact |
| `extract_while_loop` | `while true {}` → RetryFact |
| `extract_early_return` | `return;` → ExitPathFact |
| `extract_panic_exit` | Non-macro panic call → no crash |
| `extract_unreachable_call` | `unreachable_unchecked()` → no crash |
| `extract_alias` | `let r = &x;` → AliasFact |
| `extract_mut_alias` | `let r = &mut x;` → AliasFact + WriteFact |
| `lifecycle_facts_serialization` | JSON serialization round-trip |
| `lifecycle_fact_schema_version` | `LIFECYCLE_FACT_SCHEMA_VERSION = "0.1.0"` |
| `resource_kind_known` | Known resource type mapping |
| `extract_multiple_writes` | Multiple assignments → multiple WriteFacts |

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo test --workspace` | 174 tests pass (75 Rust frontend, 67 CIR, 14 conformance, 9 consumer, 4+4+1+1 fixture) |
| `cargo fmt --check` | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clean |
| `openspec validate add-rust-analysis-frontend --strict` | Valid |

## Published Contracts

- **Lifecycle-fact schema:** v0.1.0 — `vampiro_rust_frontend::lifecycle::LifecycleFacts`
- **Extraction function:** `vampiro_rust_frontend::lifecycle::extract_lifecycle_facts(syntax, path) -> LifecycleFacts`
- **Fixture path:** `tests/fixtures/add-rust-analysis-frontend/4/`