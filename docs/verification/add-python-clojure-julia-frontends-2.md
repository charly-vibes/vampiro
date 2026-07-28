# Verification: python-extraction-tracer

> Python CIR extraction and conformance — frontend implementation.

**Ticket:** `vampiro-0vb.8.2`
**Date:** 2026-07-28
**Change:** `add-python-clojure-julia-frontends`, section 2

## Summary

Created `crates/vampiro-python-frontend/` with:

- **`PythonFrontend`** — implements `Frontend` trait for Python 3.8–3.13 using tree-sitter-python
- **Extraction logic** — CIR nodes (function, class, lambda), edges (calls, method calls), effects (async, yield, try/except, with), shapes (type hints), provenance (direct, within, over-bound)
- **18 unit tests** — validates parsing, node extraction, call detection, effect detection, async handling, lambda handling, class methods, and harness conformance

## Verification commands

```bash
# Unit tests
cargo test -p vampiro-python-frontend
# → 18 passed, 0 failed

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
| Python frontend | `0.1.0` | `crates/vampiro-python-frontend/src/lib.rs` |
| Parser | `tree-sitter-python 0.25.0` | `Cargo.toml` |

## Extraction capabilities

| Category | Capabilities | Status |
|----------|-------------|--------|
| Nodes | function_definition, class_definition, lambda_expression, async_function | ✅ |
| Edges | direct_call, method_call (attribute) | ✅ |
| Effects | plain, async (async def, await), stream (yield), result (try/except), resource (with) | ✅ |
| Shapes | scalar, record (tuple, list, dict), union (T \| None) | ✅ |
| Provenance | direct (≤3 hops), within (≤10 hops), over_bound (>10 hops) | ✅ |
| Deterministic | Same input → same output | ✅ |
| Harness | Compatible with vampiro-frontend-harness | ✅ |

## RO5U findings fixed

| ID | Severity | Issue | Fix |
|----|----------|-------|-----|
| CORR-001 | HIGH | Effect detection only recursed into `block` nodes, missing nested `yield`/`await` | Changed to recurse into all children |
| CORR-002 | MEDIUM | `map_or` → `is_some_and` for async detection | Applied clippy fix |
| CORR-003 | MEDIUM | `format!("<lambda>")` → `"<lambda>".to_string()` | Applied clippy fix |
| CORR-004 | LOW | 8-argument function signature | Refactored to `EffectFlags` struct + allowed attribute |
| CORR-005 | LOW | Unnecessary `as usize` casts | Removed casts |

## Closure

All 3 checklist items (2.1 fixtures, 2.2 frontend implementation, 2.3 conformance) are complete. The Python frontend is ready for law/lifecycle/core integration (section 5).