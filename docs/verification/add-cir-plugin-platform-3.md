# Verification: Section 3 — Plugin Load and Conformance Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.2.4
**Spec:** `openspec/changes/add-cir-plugin-platform/specs/cir-plugin-platform/spec.md`

## Summary

Implemented the `Frontend` trait contract, conformance tests, and depth-limit rejection. All tests pass.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 3.1 | ✓ | Added passing fixtures for `Frontend` trait contract: language identifier, graph extraction, depth-limit rejection. Multi-plugin scenarios deferred per YAGNI decision (see `docs/decisions/plugin-boundary.md`). |
| 3.2 | ✓ | `Frontend` trait already implemented with `extract()` and `language()` methods. `CirError` types provide `Extraction`, `EffectDepthExceeded`, and `ShapeDepthExceeded` variants. Load-manifest serialization deferred per YAGNI decision. |
| 3.3 | ✓ | 14 frontend conformance tests pass: trait contract, negative fixtures (invalid source, depth exceeded), fixture-based conformance, byte reproducibility. |

## Fixtures

Located at `tests/fixtures/add-cir-plugin-platform/3/`:

| Fixture | Purpose |
|---------|---------|
| `valid-extraction.json` | Valid CirGraph (2 nodes, 1 edge, plain effects) |
| `depth-exceeded-effect.json` | Effect channel depth = 65 (exceeds 64 max) |
| `depth-exceeded-shape.json` | Shape depth = 65 (exceeds 64 max) |

## Test Suite

File: `crates/vampiro-cir/tests/fixture_frontend_tests.rs`

### Frontend trait contract tests
| Test | What it verifies |
|------|-----------------|
| `frontend_has_language_identifier` | Every frontend returns a `&'static str` language identifier |
| `frontend_language_is_static_str` | Language return type is `&'static str` (compile-time check) |
| `frontend_valid_extraction_produces_graph` | `extract()` returns a valid `CirGraph` |
| `frontend_valid_extraction_validates` | Extracted graph passes `validate()` |
| `frontend_failing_extraction_returns_error` | `extract()` propagates `CirError::Extraction` |
| `frontend_depth_exceeded_effect_is_rejected` | `validate()` rejects effect depth > 64 |
| `frontend_depth_exceeded_shape_is_rejected` | `validate()` rejects shape depth > 64 |
| `frontend_byte_reproducibility` | Same input produces byte-for-byte identical output |
| `frontend_null_contract` | `NullFrontend` satisfies the contract |

### Fixture-based conformance tests
| Test | What it verifies |
|------|-----------------|
| `fixture_valid_extraction_round_trip` | Fixture deserializes and round-trips losslessly |
| `fixture_valid_extraction_passes_validation` | Valid fixture passes `validate()` |
| `fixture_depth_exceeded_effect_is_rejected` | Depth-exceeded effect fixture is rejected |
| `fixture_depth_exceeded_shape_is_rejected` | Depth-exceeded shape fixture is rejected |
| `fixture_canonical_utf8_byte_reproducibility` | Serialization produces reproducible UTF-8 bytes |

## Mock Frontends

| Frontend | `language()` | `extract()` behavior |
|----------|-------------|---------------------|
| `MockValidFrontend` | `"mock-valid"` | Returns 2-node, 1-edge graph matching `valid-extraction.json` |
| `MockFailingFrontend` | `"mock-fail"` | Returns `Err(CirError::Extraction(...))` |
| `MockDepthExceededEffectFrontend` | `"mock-depth-effect"` | Returns graph with effect depth 65 |
| `MockDepthExceededShapeFrontend` | `"mock-depth-shape"` | Returns graph with shape depth 65 |

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo test --workspace` | 89 tests passed (67 unit + 14 frontend + 4 category + 4 round-trip) |
| `cargo test -p vampiro-cir cir_plugin_platform_3` | 14 tests passed |
| `cargo fmt --check` | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clean |

## Existing Tests Preserved

All 67 unit tests in `vampiro-cir` continue to pass, including:
- 22 effect/shape tests
- 30 category/filtration tests
- 11 CIR graph tests (round-trip, validation, depth limits)
- 4 provenance tests

All 8 existing fixture integration tests continue to pass:
- 4 CIR round-trip tests (fixture_round_trips.rs)
- 4 category/filtration tests (fixture_category_tests.rs)

## CIR Consumer Contract

- `Frontend` trait: `language() -> &'static str`, `extract(source, path) -> Result<CirGraph, CirError>`
- `CirGraph::validate()` enforces depth limits on construction
- `CirGraph::from_json()` validates on deserialization
- `NullFrontend` available as a built-in placeholder
- Fixture path: `tests/fixtures/add-cir-plugin-platform/3/`