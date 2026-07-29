# Dogfooding Run 4: `vampiro check` after per-slot argument binding

**Date:** 2026-07-29
**Ticket:** vampiro-eos
**Pipeline:** Same as dogfood-2 (`vampiro check --full --mode guidance --json`, RustFrontend → VisibilityFacts → full analyzer pipeline)
**Scope:** 6 Rust codebases (wai, dont, pretender, espectacular, testaruda, vampiro)
**Methodology:** Identical to dogfood-2 (run `vampiro check` on each repo's `src/` or equivalent, count findings by class, sample triage). Results compared against the [dogfood-2 baseline](dogfood-2.md).

## Summary

| Metric | dogfood-2 (before) | dogfood-4 (after) | Change |
|--------|-------------------:|------------------:|-------:|
| Repos scanned | 6 | 6 | — |
| Total findings | 543 | 4662 | +4119 |
| Composition-break | 194 (~100% FP) | 4284 (~100% FP) | +4090 |
| Swallowed-effect | 280 (~100% FP) | 297 | +17 |
| Redundancy-mismatch | 67 (~100% FP) | 81 | +14 |
| Facade-leak | 2 (bug) | 0 | -2 |
| False-positive rate | ~99.6% | ~100% | unchanged |

**Headline result:** the per-slot argument binding (slot-boundary check) **massively increased** the composition-break finding count without improving precision. The slot-boundary check introduces a new false-positive class that dominates every repo.

## Findings by class

| Class | Rule | Count | Triage | Root cause |
|-------|------|------:|--------|------------|
| composition-break (return-boundary) | REQ-7 | 378 | ~100% FP | Same as dogfood-2: caller.codomain vs callee.codomain on unrelated edges (method calls, `?`, in-place mutation). Unchanged. |
| composition-break (slot-boundary) | REQ-7 | 3906 | ~100% FP | **New.** Compares `caller.codomain` (the containing function's return type) against `callee.domain[slot]` (what the callee expects at that parameter). Since most containing functions return `()` (Scalar) and most callees expect non-unit parameters (references, containers, etc.), the check fires on nearly every multi-argument call — a universal false positive. |
| swallowed-effect | REQ-9 | 297 | ~100% FP | Unchanged from dogfood-2. Flagging `.unwrap()`/`.expect()` inside `#[test]` modules. |
| redundancy-mismatch | REQ-11 | 81 | ~100% FP | Unchanged. Coarse consumer-grouping model. |
| facade-leak | REQ-V7 | 0 | — | The test-module bug (vampiro-03s) no longer fires; likely because the specific files or patterns changed. |

## Per-repo breakdown

| Repo | Total | Comp (slot) | Comp (return) | Swallow | Redund | Facade |
|------|------:|------------:|--------------:|--------:|-------:|------:|
| wai | 890 | 664 | 113 | 89 | 24 | 0 |
| dont | 985 | 856 | 74 | 38 | 17 | 0 |
| pretender | 524 | 388 | 80 | 34 | 22 | 0 |
| espectacular | 656 | 495 | 31 | 122 | 8 | 0 |
| testaruda | 75 | 40 | 30 | 4 | 1 | 0 |
| vampiro | 1532 | 1463 | 50 | 10 | 9 | 0 |
| **Total** | **4662** | **3906** | **378** | **297** | **81** | **0** |

## Representative triage samples

### Slot-boundary — false positive (dominant pattern)

| Repo | Location | Pattern | Triage |
|------|----------|---------|--------|
| wai | `doctor/mod.rs:1591` slots 0-3 | `fn doctor(args)` calls `log::warn!(...)` (4 args). Caller returns `()` (Scalar). Callee expects `Ref(Scalar)` at each slot. | **FP** — containing function returns `()`, but its return type has nothing to do with what's being passed. The check compares the outer function's return type against each parameter's expected type. |
| wai | `gates.rs:256` slots 0-2 | `fn check_gates()` returns `Result<Vec<String>>`, calls `path_buf.push(...)` at 3 slots | **FP** — `Result<Vec<String>>` is the return type of the outer `check_gates` function, not the type of the value being passed to `push`. |
| dont | `events.rs:110` slots 0-5 | `fn epoch_to_parts()` returns `Record(Scalar×6)`, calls arithmetic ops | **FP** — 1st param of `+` is `Scalar`, but outer function returns a 6-tuple. The tuple return type is compared against the `+` operator's parameter type. |

**Root cause:** the slot-boundary check uses `caller.codomain` as the value being passed, but the actual argument value is an intermediate expression whose type may differ from the containing function's return type. The intermediate expression's CIR node is not connected as the edge source — the edge source is always the containing function node. Fixing this requires either:

1. **Intermediate-expression nodes** in the CIR graph for each call argument, so the edge source reflects the actual argument type.
2. **Bounding the slot check** to only fire when the edge truly represents a data-flow chain (the intermediate expression IS the function's return value, not a sub-expression).

### Return-boundary — false positive (unchanged)

Same patterns as dogfood-2: method call chains, `?` operator, in-place mutation like `.push()`, `.extend()`.

### Swallowed-effect, redundancy — false positive (unchanged)

Same root causes as dogfood-2: test-module `.unwrap()`, unrelated consumer grouping.

## Scan-mode verification

All three modes exercised on wai (890 findings, 777 medium):

| Mode | Severity threshold | Exit code | Behavior |
|------|--------------------|----------:|---------|
| guidance | — | 0 | reports all findings, never fails |
| tiered | — | 0 | reports findings tiered, no gate |
| gate | medium (default) | 3 | findings at/above threshold fail |
| gate | high | 0 | no high-severity findings → passes |

Exit codes correct. Gate mode at default threshold fails every repo due to the massive slot-boundary FP count — making gate mode unusable.

## Update: slot-boundary redesign (vampiro-51v.2)

The slot-boundary check was redesigned in the same session. Instead of comparing the containing function's return type (`caller.codomain`) against the callee's domain slot (which produced 3906 false positives), the frontend now computes the **actual argument expression shape** and stores it on the edge as `arg_shape`. When `arg_shape` is available, the check compares it against the callee's domain slot; when unavailable (unknown expression type), the check is skipped entirely.

### Redesign results

| Metric | dogfood-4 (before) | dogfood-8 (after) | Change |
|--------|-------------------:|------------------:|-------:|
| Total findings | 4662 | 2010 | −57% |
| Composition-break | 4284 | 1661 | −61% |
| Slot-boundary | 3906 | 1323 | −66% |
| Return-boundary | 378 | 338 | −11% |
| Swallowed-effect | 297 | 280 | — |
| Redundancy | 81 | 69 | — |

### Where the remaining slot-boundary FPs come from

The 1323 remaining slot-boundary findings are all a single pattern: `produced=Scalar, expected=Ref(Scalar)`. This is a **coarse shape model limitation**: string literals are mapped to `Shape::Scalar`, while `&str` parameters are `Shape::Ref(Box::new(Shape::Scalar))`. The checker fires because `Scalar ≠ Ref(Scalar)`, but string literals are valid `&str` arguments in Rust. Fixing this requires refinements to the shape model (e.g., adding a `Str` variant or a `Literal` classification).

### Acceptance criteria verdict

**Target: <80% FP rate for composition-break findings.** Not met for the coarse tracers (still ~100% FP in aggregate), but the slot-boundary redesign eliminated the dominant false-positive class (caller-codomain noise). The remaining composition FPs are now from the return-boundary check (unchanged) and the shape model's inability to distinguish string literals from refs.

## Recommended next investments

1. **Refine shape model** — add a `Shape::Str` variant (or parameterized string type) so string literals match `&str` parameters. Would eliminate virtually all remaining slot-boundary FPs.

2. **Data-flow edges in CIR** (0vb.4.7 — deferred) — true data-flow edges remain the foundational fix for all composition tracers.

3. **Test-module awareness** (vampiro-03s) — suppress swallow-effect findings inside `#[cfg(test)]` / `*_tests` modules.

4. **Guidance-only shipping** — the coarse tracers remain unusable in `gate` mode. Guidance mode only.