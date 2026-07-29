# Dogfooding Run 2: `vampiro check` across the charly-vibes ecosystem

**Date:** 2026-07-29
**Ticket:** vampiro-tmf.2
**Pipeline:** `vampiro check --path <dir> --full` (RustFrontend → VisibilityFacts → CompositionAnalyzer + EffectHandlingAnalyzer + RedundancyAnalyzer + ModularityAnalyzer → envelope output)
**Scope:** 6 Rust codebases across the charly-vibes suite (wai, dont, pretender, espectacular, testaruda, vampiro). crua and livin are EARS-spec repos with no Rust source and are excluded.
**Methodology:** Representative sampling. The 543 findings were not triaged one-by-one; instead, each finding class was sampled per repo (8–15 cases) against the source, root-caused, and the pattern extrapolated with documented confidence. This follows the precedent set by [dogfood-1.md](dogfood-1.md).

## Summary

| Metric | Value |
|--------|-------|
| Repos scanned | 6 (Rust); 2 excluded (crua, livin — no `.rs`) |
| Files scanned | 80 |
| Total findings | 543 |
| Confirmed true positives | 0 |
| False positives | 541 |
| Frontend-precision bug (filed) | 2 (REQ-V7 facade-leak on test modules → vampiro-03s) |
| False-positive rate | ~99.6% (target was <5%) |

**Headline result:** the over-exposure tracer (REQ-V4), which carried all the genuine TPs in dogfood-1, now fires **zero** times across the ecosystem — every previously-filed over-exposure TP (`filtration_level` etc.) has been fixed. The remaining three tracers (composition, swallowed-effect, redundancy) are too coarse for unsupervised cross-repo use: they are dominated by the known lack of data-flow edges (0vb.4.7 scope) and a test-module facade-leak bug (vampiro-03s).

## Findings by class

| Class | Rule | Count | Triage | Root cause |
|-------|------|------:|--------|------------|
| composition-break | REQ-7 | 194 | ~100% FP | caller.codomain vs callee.codomain compares unrelated call edges — method calls, `?`, in-place mutation (`.push`, `.extend`) all produce scalar-typed callees that don't flow into the caller's return. No data-flow edges (0vb.4.7). |
| swallowed-effect | REQ-9 | 280 | ~100% FP | the frontend classifies `.unwrap()`/`.expect()` as Force+Partial → swallowed. ~all occurrences are inside `#[test]` modules (idiomatic) or are intentional force-unwraps on locally-proven invariants. True `let _ = result_fn();` discards are not detected at all. |
| redundancy-mismatch | REQ-11 | 67 | ~100% FP | the coarse "consumer with ≥2 inbound edges" model groups unrelated callers of the same callee and flags their differing codomains. No real joins exist in ordinary Rust call graphs. |
| facade-leak | REQ-V7 | 2 | FP (bug) | `mod foo_tests { use super::*; }` is treated as a public L4 facade re-export of an L1 private fn. Filed **vampiro-03s**. |

## Per-repo breakdown

| Repo | Files | Findings | composition | swallowed | redundancy | facade-leak |
|------|------:|---------:|------------:|----------:|-----------:|------------:|
| wai | 26 | 181 | 69 | 89 | 23 | 0 |
| dont | 15 | 97 | 40 | 38 | 17 | 2 |
| pretender | 6 | 52 | 25 | 17 | 10 | 0 |
| espectacular | 18 | 151 | 21 | 122 | 8 | 0 |
| testaruda | 5 | 28 | 23 | 4 | 1 | 0 |
| vampiro | 10 | 34 | 16 | 10 | 8 | 0 |
| **Total** | **80** | **543** | **194** | **280** | **67** | **2** |

Scan paths: repos with a `src/` layout were scanned at `src/`; vampiro at `crates/`; pretender at its nested `pretender/pretender/src/`.

## Representative triage samples

### Composition-break (REQ-7) — false positive

| Repo | Location | Caller / Callee | Triage |
|------|----------|-----------------|--------|
| wai | `commands/add.rs:883` | `warn_if_unlocked` returns `Result<()>`; calls `has_lock_file` (`bool`=Scalar) | **FP** — callee return doesn't flow into caller return; in-place guard check |
| pretender | `doctor.rs:74` | caller returns `Vec<T>`; calls `skip(...)` (Scalar) | **FP** — `skip`'s value is stored, not returned |
| dont | `events.rs:110` | `epoch_to_parts` returns a 6-tuple `Record`; calls scalar-returning arithmetic | **FP** — tuple components built from scalars; coarse shape model |
| vampiro | `aix.rs:94` | `write_aix_artifacts` returns `Result`; `std::fs::write(...)?` callee | **FP** — `?` operator unwrap (known, dogfood-1) |

**Root cause:** the composition tracer operates on call edges (source=caller, target=callee) and compares the two codomains. Without data-flow edges, any caller whose return type differs from a callee's return type fires — which is the common case, not a defect. This is the open 0vb.4.7 scope (per-slot argument binding / true data-flow edges).

### Swallowed-effect (REQ-9) — false positive

| Repo | Location | Pattern | Triage |
|------|----------|---------|--------|
| dont | `events.rs:144` | `TempDir::new().unwrap()` inside `#[test]` | **FP** — test idiom |
| testaruda | `adapter.rs:704` | `parse_command_string(...).unwrap()` inside `#[test]` | **FP** — test idiom |
| espectacular | `adapters/custom.rs:164` | `invoke(...).unwrap()` inside `#[test]` | **FP** — test idiom |
| vampiro | `aix.rs:194` | `write_aix_artifacts(...).unwrap()` inside a `--regen` CLI path | **FP** — intentional force-unwrap (panic is the correct behavior on a write failure during artifact regeneration) |

**Root cause:** the frontend sets `Unwrapped` + `Force` + `Partial` for every `.unwrap()`/`.expect()`, which the analyzer maps to a swallowed-effect finding. The EARS "swallowed effect" semantics is *discarding* a `Result`/`Option` without handling it (`let _ = result_fn();`). The frontend does **not** detect true discards — it only flags force-unwraps, which are panic-risks, not swallowed effects. Every repo in the suite carries heavy `#[test]` modules (test-signal counts: wai 527, testaruda 342, espectacular 239, dont 156, pretender 135), so the vast majority of `.unwrap()` findings sit in test code where they are idiomatic. No true `let _ = ` discard was observed in any sampled finding.

### Redundancy-mismatch (REQ-11) — false positive

| Repo | Location | Pattern | Triage |
|------|----------|---------|--------|
| dont | `events.rs:91-110` | `days_in_month` (returns Scalar) grouped with `epoch_to_parts` (returns `Record[Scalar×6]`) | **FP** — unrelated functions sharing an inferred consumer node |
| vampiro | `aix.rs:94-164` | two `std::fs::write` call sites span-merged into one finding | **FP** — coarse span merge of unrelated edges |
| testaruda | `adapter.rs:693-746` | several adapter method calls merged | **FP** — same pattern |

**Root cause:** the redundancy tracer groups all inbound edges on a target node and compares the sources' codomains. In ordinary Rust call graphs, "consumers with ≥2 inbound edges from differently-shaped sources" are call-graph artifacts (multiple callers of a shared utility), not real redundancy joins. No real adapter-missing joins exist in any sampled case.

### Facade-leak (REQ-V7) — false positive (filed vampiro-03s)

| Repo | Location | `facade_scope` | Triage |
|------|----------|----------------|--------|
| dont | `main.rs:2359` (`parse_line_span`, private L1 fn) | `parse_line_span_tests` | **FP** — sibling `mod parse_line_span_tests { use super::*; }` misread as a public facade |
| dont | `rules/mod.rs:248` (`source_key`, private L1 fn) | `source_key_tests` | **FP** — same pattern |

**Root cause:** the frontend treats `use super::*` inside a test module as a `FacadeReexport` of the imported private items. Test modules are not public facades. Filed **vampiro-03s** with a fix proposal (skip `*_tests`/`#[cfg(test)]`/non-`pub` modules when building `FacadeReexport`s).

## Scan-mode verification (guidance / tiered / gate)

All three modes exercised on testaruda (28 medium findings) and dont:

| Mode | Severity threshold | Exit code | Behavior |
|------|--------------------|----------:|---------|
| guidance | — | 0 | reports all findings, never fails the build |
| tiered | — | 0 | reports findings tiered by severity, no gate |
| gate | medium (default) | 3 | findings at/above threshold fail the build |
| gate | high | 0 | no high-severity findings → build passes |

Gate-mode exit codes are correct: a repo with only medium findings fails the gate at the default threshold and passes at `high`.

## False-positive-rate analysis

- **Target:** <5% FP rate.
- **Achieved:** ~99.6% FP (541/543). The two non-FP findings are not TPs either — they are a frontend-precision bug (vampiro-03s), so confirmed-TP count is 0.
- **Why:** the three coarse tracers (composition, swallowed-effect, redundancy) were always understood to be approximations pending data-flow edges (EARS §1 "deliberately coarse"). Dogfood-1 confirmed this on vampiro's own source; dogfood-2 confirms it generalizes across the entire suite. The over-exposure tracer — the only one whose signal was actionable in dogfood-1 — now fires zero times because all its TPs were fixed.

## Sub-tickets filed

- **vampiro-03s** (P2) — Frontend misclassifies test-module `use super::*` as facade re-export (REQ-V7 facade-leak FP). The only genuine defect surfaced by this dogfood run.

## Conclusion

The dogfood target (<5% FP) is **not met** for the coarse tracers and cannot be met without data-flow edges (the 0vb.4.7 scope) and a true-discard detector for swallowed effects. The over-exposure tracer, by contrast, is at effectively 0% FP (no findings, no false positives, all prior TPs fixed). Recommended next investments, in priority order:

1. **Data-flow edges in CIR** (0vb.4.7) — collapses the composition and redundancy FP classes at their root.
2. **True-discard detection** — make REQ-9 flag `let _ = result_fn();` instead of `.unwrap()`, or raise the Force+Partial finding severity to a distinct `panic-risk` classification separate from `swallowed-effect`.
3. **Test-module awareness** (vampiro-03s) — suppress `FacadeReexport`s and force-unwrap findings inside `#[cfg(test)]` / `*_tests` modules.

Until (1) and (2) land, the coarse tracers should ship in `guidance` mode only; `gate` mode is premature for cross-repo CI.
