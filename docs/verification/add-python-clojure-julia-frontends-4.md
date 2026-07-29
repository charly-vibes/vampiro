# Verification: julia-extraction-tracer

> Julia CIR extraction and conformance — frontend implementation.

**Ticket:** `vampiro-0vb.8.4`
**Date:** 2026-07-28
**Change:** `add-python-clojure-julia-frontends`, section 4

## Summary

Created `crates/vampiro-julia-frontend/` with:

- **`JuliaFrontend`** — implements `Frontend` trait for Julia 1.6–1.11 using tree-sitter-julia
- **Extraction logic** — CIR nodes (function, macro, struct, module, arrow function), edges (call, broadcast, macrocall), effects (try/catch, @async/@sync macros)
- **13 unit tests** — validates parsing, node extraction, call detection, effect detection, struct, macro, module, and harness conformance

## Verification commands

```bash
cargo test -p vampiro-julia-frontend         # 13 passed
cargo test --workspace                        # all pass
cargo clippy --workspace --all-targets -- -D warnings  # clean
cargo fmt --check                             # clean
openspec validate add-python-clojure-julia-frontends --strict  # valid
```

## Extraction capabilities

| Category | Capabilities | Status |
|----------|-------------|--------|
| Nodes | function_definition, macro_definition, struct_definition, module_definition, arrow_function_expression | ✅ |
| Edges | call_expression, broadcast_call_expression, macrocall_expression | ✅ |
| Effects | async (@async/@sync), result (try/catch) | ✅ |
| Provenance | direct (≤3 hops), within (≤10 hops), over_bound (>10 hops) | ✅ |
| Harness | Compatible with vampiro-frontend-harness | ✅ |

## RO5U findings fixed

| ID | Severity | Issue | Fix |
|----|----------|-------|-----|
| CORR-001 | HIGH | `find_decl_name` didn't look inside `signature` → `call_expression` | Walk into signature for identifiers |
| CORR-002 | MEDIUM | Clippy: `len() >= 1`, `only_used_in_recursion`, formatting | Applied fixes and allow attributes |
| CORR-003 | LOW | Test source strings with `$` interpolation | Simplified test sources |

## Closure

All 3 checklist items (4.1 fixtures, 4.2 frontend, 4.3 conformance) are complete. Julia frontend is ready for law/lifecycle/core integration (section 7).