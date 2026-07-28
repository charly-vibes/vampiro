# Verification: Section 1 — Base Rust CIR Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.3.2
**Spec:** `openspec/changes/add-rust-analysis-frontend/specs/cir-plugin-platform/spec.md`

## Summary

Delivered the base Rust CIR tracer using `syn` as the parser. Parses Rust source, extracts function declarations as CirNodes (with shapes, effects, source spans), and extracts intra-file call edges between known functions.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 1.1 | ✓ | Added 3 fixtures for callable nodes, structural/opaque shapes, recursive effects, and source spans. |
| 1.2 | ✓ | Implemented syntax-to-CIR extraction using `syn` under the approved parser boundary. No source execution. |
| 1.3 | ✓ | 20 unit tests pass. Fixtures and consumer test published. |

## Implementation

Parser: `crates/vampiro-rust-frontend/src/lib.rs` — implements `vampiro_cir::Frontend` for Rust.

Extraction: `crates/vampiro-rust-frontend/src/extract.rs` — walks syn AST with `Visit` trait:

| Feature | Implementation |
|---------|---------------|
| Function declarations | `visit_item_fn` → `CirNode` with domain/codomain shapes |
| Function calls | `visit_expr_call` + `visit_expr_method_call` → `CirEdge` with known callee resolution |
| Shape extraction | `fn extract_shape` — maps syn types to `Shape` (Scalar, Ref, Record, Parameterized, Opaque) |
| Effect extraction | `fn extract_effect_from_type` — recognizes Result, Option, async as EffectChannel |
| Unwrap detection | `fn detect_unwrap` — recognizes `unwrap()`, `expect()`, `unwrap_unchecked()` |
| Source spans | `fn make_span` — maps syn span to `SourceSpan` |
| Call resolution | Tracks current function context; only creates edges for known callees |

## Fixtures

Located at `tests/fixtures/add-rust-analysis-frontend/1/`:

| Fixture | Rust Source | Nodes | Edges | Effect |
|---------|------------|-------|-------|--------|
| `simple-function.json` | `fn hello() -> i32 { 42 }` | 1 | 0 | Plain |
| `complex-function.json` | `fn process(a: i32, b: &str) -> Result<String, Error>` | 1 | 0 | Result |
| `async-effect.json` | `async fn fetch() -> Result<String, Error>` | 1 | 0 | Recursive(Result) |

## Test Suite

File: `crates/vampiro-rust-frontend/src/lib.rs` (5 tests) + `crates/vampiro-rust-frontend/src/extract.rs` (15 tests)

### Frontend contract tests
| Test | What it verifies |
|------|-----------------|
| `rust_frontend_language` | `language()` returns `"rust"` |
| `rust_frontend_language_is_static` | Language is `&'static str` |
| `parses_empty_source` | Empty source produces empty graph |
| `parses_simple_function` | Function declaration produces one node |
| `parses_function_with_async_effect` | `async fn` produces `Recursive(Result)` effect |
| `parses_function_with_calls` | Function calls produce edges |
| `rejects_invalid_rust` | Invalid Rust returns `CirError::Extraction` |

### Extraction tests
| Test | What it verifies |
|------|-----------------|
| `extract_simple_function_no_params` | No params → `Shape::Scalar` domain |
| `extract_function_with_params` | Two params → `Shape::Record` domain |
| `extract_async_function` | `async fn` → `Recursive(Plain)` effect |
| `extract_function_with_result` | `Result` return → `EffectChannel::Result` |
| `extract_function_call` | Known function call → edge with `Propagated` resolution |
| `extract_fully_qualified_call_skipped` | `crate::helper()` not matched (future enhancement) |
| `extract_shape_ref` | `&str` → `Shape::Ref` |
| `extract_shape_parameterized` | `Vec<i32>` → `Shape::Parameterized` |
| `extract_span_information` | Source spans have line numbers |
| `extract_empty_file` | Empty file → empty graph |
| `extract_only_comments` | Only comments → empty graph |
| `extract_multiple_functions` | 3 functions → 3 nodes |
| `extract_graph_validates` | Extracted graph passes `validate()` |

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo test --workspace` | 118 tests pass (20 new Rust frontend tests) |
| `cargo fmt --check` | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clean |
| `openspec validate add-rust-analysis-frontend --strict` | Valid |

## CIR Consumer Contract

- **Frontend:** `RustFrontend` implements `vampiro_cir::Frontend`
- **Language:** `"rust"`
- **Parser:** `syn` v2, Rust 2021+ edition
- **Extraction:** Function declarations → CirNode, known function calls → CirEdge
- **Exclusions:** Stdlib/builtin calls, cross-module qualified calls, macro expansion
- **Fixture path:** `tests/fixtures/add-rust-analysis-frontend/1/`
- **Crate:** `vampiro-rust-frontend` v0.1.0