# Verification: clojure-extraction-tracer

> Clojure CIR extraction and conformance — frontend implementation.

**Ticket:** `vampiro-0vb.8.3`
**Date:** 2026-07-28
**Change:** `add-python-clojure-julia-frontends`, section 3

## Summary

Created `crates/vampiro-clojure-frontend/` with:

- **`ClojureFrontend`** — implements `Frontend` trait for Clojure 1.10–1.12 using tree-sitter-clojure
- **Extraction logic** — CIR nodes (defn, fn, def, defmulti, defmethod, defprotocol, defrecord, deftype), edges (function calls), effects (future, lazy-seq, try/catch, with-open, binding), reader macros as opaque
- **16 unit tests** — validates parsing, node extraction, call detection, effect detection, anonymous functions, multimethods, and harness conformance

## Verification commands

```bash
cargo test -p vampiro-clojure-frontend       # 16 passed
cargo test --workspace                        # all pass
cargo clippy --workspace --all-targets -- -D warnings  # clean
cargo fmt --check                             # clean
openspec validate add-python-clojure-julia-frontends --strict  # valid
```

## Extraction capabilities

| Category | Capabilities | Status |
|----------|-------------|--------|
| Nodes | defn, defn-, fn, def, defmulti, defmethod, defprotocol, defrecord, deftype, anon_fn_lit #() | ✅ |
| Edges | Function calls (list_lit with symbol operator) | ✅ |
| Effects | async (future), stream (lazy-seq), result (try/catch), resource (with-open, binding) | ✅ |
| Reader macros | Quoting, syntax-quoting, unquoting, deref, metadata, read-cond | ✅ (opaque) |
| Provenance | direct (≤3 hops), within (≤10 hops), over_bound (>10 hops) | ✅ |
| Harness | Compatible with vampiro-frontend-harness | ✅ |

## RO5U findings fixed

| ID | Severity | Issue | Fix |
|----|----------|-------|-----|
| CORR-001 | HIGH | `get_symbol_text` used full node text instead of `sym_name` child | Walk into `sym_name` child for actual text |
| CORR-002 | HIGH | `scan_clojure_effects` passed `""` as source to `get_symbol_text` | Pass actual source string |
| CORR-003 | MEDIUM | `def` handler hardcoded `EffectChannel::Plain` | Use `detect_clojure_effect` |
| CORR-004 | MEDIUM | List operator at wrong child index (0 vs 1) | Use `child(1)` after `(` |
| CORR-005 | LOW | Clippy: `map_or`, `as_deref`, `format!` borrows, `too_many_arguments` | Applied fixes and allow attributes |

## Closure

All 3 checklist items (3.1 fixtures, 3.2 frontend, 3.3 conformance) are complete. Clojure frontend is ready for law/lifecycle/core integration (section 6).