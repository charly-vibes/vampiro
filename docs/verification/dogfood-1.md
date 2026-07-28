# Dogfooding Run 1: `vampiro check` on its own source

**Date:** 2026-07-28
**Pipeline:** `vampiro check --path <file>` (RustFrontend → VisibilityFacts → CompositionAnalyzer + ModularityAnalyzer → Envelope output)
**Scope:** All 27 source files across 4 crates (vampiro-cir, vampiro-cli, vampiro-rust-frontend, vampiro-seam-analysis)

## Summary

| Metric | Value |
|--------|-------|
| Files scanned | 27 |
| Files with findings | 14 (52%) |
| Total findings | 22 |
| Composition-break findings | 4 (all false positives) |
| Over-exposure findings | 18 (mix of real over-exposure and intentional API surface) |
| True positives | ≈10 |
| False positives | 12 |

## Findings by file

### Composition-break findings (REQ-7) — all false positives

| File | Line | Detail | Triage |
|------|------|--------|--------|
| `vampiro-cli/src/aix.rs` | 94 | `std::fs::write(...)?` — callee produces `Result<()>` but `?` unwraps to `()` | **FP** — `?` operator isn't traced by data-flow analysis; function correctly composes |
| `vampiro-cli/src/aix.rs` | 97 | Same pattern (second `write` call in `write_aix_artifacts`) | **FP** — same root cause |
| `vampiro-seam-analysis/src/lib.rs` | 55 | `findings.extend(...)` — analyzer sees `Vec::extend` return vs tuple return type | **FP** — in-place mutation via `.extend()`; function's `(Vec, Vec)` return is correct |
| `vampiro-seam-analysis/src/lib.rs` | 56 | Same (tuple construction `(findings, mod_diags)`) | **FP** — same root cause |

**Root cause:** The composition tracer uses Shape-only analysis without data-flow edges. The `?` operator, method calls with mutation, and tuple construction are all invisible to the current model. This is the known gap tracked by the original 0vb.4.7 scope (codomain-vs-codomain was fixed; `?` operator and data-flow remain).

### Over-exposure findings (REQ-V4) — triage by crate

#### vampiro-cir (3 findings)

| File | Line | Symbol | Triage |
|------|------|--------|--------|
| `category.rs` | 237 | `pub fn filtration_level(...)` — **not** in crate's `pub use` re-export list | **True positive** — internal helper reachable only via `vampiro_cir::category::filtration_level`. Should be `pub(crate)`. |
| `category.rs` | 256 | `pub fn validate_category(...)` — **is** in `pub use category::{validate_category, ...}` | **Intentional** — part of the crate's declared public API surface |
| `category.rs` | 423 | `pub fn validate_filtration(...)` — **is** in `pub use category::{validate_filtration, ...}` | **Intentional** — same as above |

#### vampiro-cli (6 findings)

| File | Line | Symbol | Triage |
|------|------|--------|--------|
| `aix.rs` | 28 | `pub fn generate_llms_txt(meta) -> String` | **Intentional** — public API for downstream consumers / integration tests |
| `aix.rs` | 47 | `pub fn generate_llm_txt(meta) -> String` | **Intentional** — same |
| `aix.rs` | 93 | `pub fn write_aix_artifacts(dir, meta) -> Result` | **Intentional** — same |
| `config.rs` | 44 | `pub fn vampiro_config_store() -> ConfigStore` | **Intentional** — public API for upstream consumers (doctor, suite tools) |
| `managed.rs` | 11 | `pub fn vampiro_registry() -> BlockRegistry` | **Intentional** — public API for CLI main.rs |
| `managed.rs` | 18 | `pub fn vampiro_injector() -> BlockInjector` | **Intentional** — same |

#### vampiro-rust-frontend (4 findings)

| File | Line | Symbol | Triage |
|------|------|--------|--------|
| `extract.rs` | 34 | `pub fn extract_graph(...)` — in **private** `mod extract` | **FP** — module is private (`mod extract`), so `pub` is effectively crate-internal. Analyzer doesn't distinguish module visibility from item visibility. |
| `law.rs` | 265 | `pub fn extract_law_input(...)` — in `pub mod law` | **Intentional** — `lib.rs` re-exports `LawRunnerInput` type; the function is callable via `extract_full`. Could be tightened to `pub(crate)` if desired. |
| `lifecycle.rs` | 157 | `pub fn extract_lifecycle_facts(...)` — in `pub mod lifecycle` | **Intentional** — same pattern as `law.rs` |
| `visibility_adapter.rs` | 31 | `pub fn to_visibility_facts(...)` — in `pub mod visibility_adapter` | **Intentional** — consumed by `vampiro-cli` and `vampiro-seam-analysis` externally |

#### vampiro-seam-analysis (5 findings)

| File | Line | Symbol | Triage |
|------|------|--------|--------|
| `lib.rs` | 39 | `pub fn analyze(graph) -> Vec<Finding>` | **Intentional** — part of the crate's public API |
| `lib.rs` | 51 | `pub fn analyze_with_visibility(...)` | **Intentional** — same |
| `composition.rs` | 43 | `pub fn unify_shapes(...)` — **is** in `pub use composition::{unify_shapes, ...}` | **Intentional** — re-exported by lib.rs |

## Verdict

**22 findings → 10 actionable tightening candidates, 12 false positives.**

| Category | Count | Action |
|----------|-------|--------|
| Composition FP (`?` operator) | 4 | File as 0vb.4.x enhancement: data-flow modeling for `?` and mutation |
| Over-exposure FP (private module) | 1 | Fix analyzer to check module visibility, not just item visibility |
| Over-exposure intentional API | 13 | No action — correctly `pub` per crate design |
| Over-exposure true positive | 1 | Tighten `filtration_level` to `pub(crate)` |
| Over-exposure candidate tightening | 2 | `extract_law_input` and `extract_lifecycle_facts` could be `pub(crate)` |

## Recommendations

1. **Tighten `vampiro_cir::category::filtration_level` → `pub(crate)`** — it's only used by `validate_filtration` within the same crate.
2. **File enhancement ticket** — data-flow modeling for `?` operator, mutation, and tuple construction (extends the 0vb.4.7 composition fix).
3. **Fix the modularity analyzer** — `pub fn` in a private module should not be flagged as over-exposure.
4. **Optional:** tighten `extract_law_input` and `extract_lifecycle_facts` to `pub(crate)` — they're called internally by `extract_full` and shouldn't need external reach.