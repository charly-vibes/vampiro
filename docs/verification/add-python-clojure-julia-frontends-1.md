# Verification: shared-frontend-harness-1

> Shared CIR acceptance contract — per-language matrices and harness.

**Ticket:** `vampiro-0vb.8.1`
**Date:** 2026-07-28
**Change:** `add-python-clojure-julia-frontends`, section 1

## Summary

Created `crates/vampiro-frontend-harness/` with:

- **`LanguageMatrix`** — per-language extraction matrix defining node, edge, shape, effect, provenance, and visibility capabilities (REQ-1–3, REQ-V1–V2).
- **`CompatibilityHarness`** — versioned platform compatibility harness that consumes only published CIR/plugin contracts (`vampiro-cir::Frontend` trait) and can run independently for each language.
- **`ConformanceReport`** — structured JSON output with per-entry Passed/Failed/Skipped results.
- **Built-in matrices** — `python_matrix()`, `clojure_matrix()`, `julia_matrix()` with full capability coverage.
- **`run_empty()`** — reference harness against `NullFrontend` for the empty-harness phase.

## Verification commands

```bash
# Unit tests
cargo test -p vampiro-frontend-harness
# → 11 passed, 0 failed

# Full workspace
cargo test --workspace
# → all tests pass

# Clippy
cargo clippy --workspace --all-targets -- -D warnings
# → clean

# Formatting
cargo fmt --check
# → clean

# OpenSpec validation
openspec validate add-python-clojure-julia-frontends --strict
# → valid
```

## Key contracts

| Contract | Version | Location |
|----------|---------|----------|
| CIR graph | `0.1.0` | `vampiro-cir` |
| Language matrix | `0.1.0` | `crates/vampiro-frontend-harness/src/lib.rs` |
| Conformance report | `0.1.0` | `crates/vampiro-frontend-harness/src/lib.rs` |

## Matrices

### Python (4 nodes, 2 edges, 3 shapes, 5 effects, 3 provenances, 3 visibilities)
- Nodes: `function_declaration`, `class_declaration`, `lambda_expression`, `async_function`
- Edges: `direct_call`, `method_call`
- Shapes: `scalar`, `record`, `union`
- Effects: `plain`, `async`, `option`, `result`, `stream`
- Provenance: `direct`, `within`, `over_bound`
- Visibility: `public`, `private`, `facade`

### Clojure (4 nodes, 2 edges, 3 shapes, 6 effects, 3 provenances, 3 visibilities)
- Nodes: `function_declaration`, `anonymous_function`, `protocol_method`, `multimethod`
- Edges: `direct_call`, `method_call` (Java interop)
- Shapes: `scalar`, `record`, `union`
- Effects: `plain`, `async`, `option`, `result`, `stream`, `resource`
- Provenance: `direct`, `within`, `over_bound`
- Visibility: `public`, `private`, `facade`

### Julia (4 nodes, 3 edges, 3 shapes, 6 effects, 3 provenances, 3 visibilities)
- Nodes: `function_declaration`, `anonymous_function`, `struct_declaration`, `macro_declaration`
- Edges: `direct_call`, `method_call` (multiple dispatch), `broadcast_call`
- Shapes: `scalar`, `record`, `union`
- Effects: `plain`, `async`, `option`, `result`, `stream`, `resource`
- Provenance: `direct`, `within`, `over_bound`
- Visibility: `public`, `private`, `facade`

## RO5U findings fixed

| ID | Severity | Issue | Fix |
|----|----------|-------|-----|
| CORR-001 | HIGH | Empty-graph check only applied to node/edge categories | Generalized to all categories |
| CORR-002 | MEDIUM | Misleading `_frontend` prefix on used parameter | Renamed to `frontend` |
| CORR-003 | HIGH | Determinism test returned Passed when all samples failed to parse | Now returns Failed if no samples testable |
| EDGE-002 | MEDIUM | serde_json failure silently produced empty strings for comparison | Now returns explicit Failed |

## Closure

All 3 checklist items (1.1 matrices, 1.2 harness, 1.3 empty harness) are complete. The harness is ready for sections 2–4 (Python, Clojure, Julia frontend implementations).