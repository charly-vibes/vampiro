# Verification: Section 3 — Law Runner-Input Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.3.4
**Spec:** `openspec/changes/add-rust-analysis-frontend/tasks.md#3`

## Summary

Delivered law runner-input extraction. Extracts implementation clusters, proof/law tagged functions, serializable values, and generator references. Runner-input schema v0.1.0.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 3.1 | ✓ | Added fixtures for implementation clusters, proof/law tags, serializable values, generator references. |
| 3.2 | ✓ | Implemented runner-input extraction only (no runner execution). Extraction owned by law verification module. |
| 3.3 | ✓ | 14 tests pass. Runner-input schema v0.1.0 published. |

## Implementation

**Module:** `crates/vampiro-rust-frontend/src/law.rs`

| Type | Schema Version | Description |
|------|---------------|-------------|
| `LawRunnerInput` | 0.1.0 | Top-level container for all runner-input data |
| `ImplCluster` | — | `impl` block with self_type, trait_name, methods |
| `TaggedFn` | — | Function with `#[law]`, `#[proof]`, `#[test]` tags, params, return type |
| `FnParam` | — | Parameter name, type string, serializability flag |
| `SerializableValue` | — | Variable with type and serializability info |
| `GeneratorRef` | — | Iterator/stream/generator reference with item type |

## Fixtures

Located at `tests/fixtures/add-rust-analysis-frontend/3/`:

| Fixture | Content |
|---------|---------|
| `law-tagged-fn.json` | `#[law] fn check_property(n: i32, s: String) -> bool` |
| `impl-cluster.json` | `impl Foo { fn bar() {} fn baz() {} }` |

## Test Suite

### Law runner-input tests (14 new)
| Test | What it verifies |
|------|-----------------|
| `empty_file_produces_empty_input` | Empty source → empty input |
| `extract_simple_impl_block` | `impl Foo { fn bar() {} fn baz() {} }` → 1 cluster, 2 methods |
| `extract_trait_impl` | `impl Trait for Foo` → trait_name, is_trait_impl |
| `extract_empty_impl_skipped` | `impl Foo {}` → no cluster |
| `extract_tagged_fn_law` | `#[law] fn` → tag "law" |
| `extract_tagged_fn_test` | `#[test] fn` → tag "test" |
| `extract_untagged_fn_skipped` | Normal fn → no tagged fn |
| `extract_tagged_fn_params` | Params with type names and serializability |
| `extract_serializable_type` | Known serializable types |
| `extract_generator_from_let` | `let iter: Iterator<i32>` → no crash |
| `extract_multiple_impl_blocks` | Multiple impl blocks |
| `extract_type_name` | Type name string conversion |
| `runner_input_serialization` | JSON serialization round-trip |
| `runner_input_schema_version` | `RUNNER_INPUT_SCHEMA_VERSION = "0.1.0"` |

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo test --workspace` | 161 tests pass (62 Rust frontend, 67 CIR, 14 conformance, 9 consumer, 4+4+1 fixture) |
| `cargo fmt --check` | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clean |
| `openspec validate add-rust-analysis-frontend --strict` | Valid |

## Published Contracts

- **Runner-input schema:** v0.1.0 — `vampiro_rust_frontend::law::LawRunnerInput`
- **Extraction function:** `vampiro_rust_frontend::law::extract_law_input(syntax, path) -> LawRunnerInput`
- **Fixture path:** `tests/fixtures/add-rust-analysis-frontend/3/`