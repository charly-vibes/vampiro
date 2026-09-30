# Dogfooding Run 5: the dulce-de-leche ecosystem (foreign-code value-proposition check)

**Date:** 2026-09-30
**Tickets:** vampiro-224 (epic) + vampiro-224.3 … vampiro-224.8
**Pipeline:** `vampiro check --full --mode guidance --json` (vampiro 0.4.0, all 8 frontends, data-flow edges, is_test filtering)
**Scope:** all 8 Rust repos of the dulce-de-leche family: `wai`, `dont`, `dulce-de-leche`, `espectacular`, `pretender`, `testaruda`, `vampiro`, `fotos` (fabbro excluded — Go)

## Outcome (updated after the vampiro-224.3 fix, same day)

R1 (unresolvable named types → Opaque, plus nested-opaque/bottom-aware
`unify_shapes` exclusion, `Result<T>` alias arity, and effect-wrapper-vs-leaf
exclusion) landed immediately after the triage:

| Metric | Pre-fix | Post-R1 | Δ |
|---|---:|---:|---:|
| Total findings (original 8 repos) | 515 | **299** | −42% |
| Composition-break (original 8 repos) | 393 | **185** | −53% |
| All 5 acceptance-site composition-breaks | flagged | **clear** | ✅ |
| Workspace tests | 824 | 831 | +9 new tests |
| Seeded-fixture TPs (soundness + precision) | pass | pass | ✅ no TP loss |

Per-repo, per-classification counts for both runs: `dogfood-5-corpus.json`
(next to this file). **Post-fix FP rate is unmeasured** — the 185 remaining
findings were not re-triaged as a set; the next round must re-triage before
claiming progress toward the <5% target.

Composition remainder maps to: 96 unit-shape (genuine `()` returns at the
return-boundary approximation), 36 unknown-involving (heuristic misses), 53
resolved (R2–R5 classes: condition slots, rewraps, combinators). Redundancy
noise (108) is vampiro-224.8.

---

## Original pre-fix results

## Why this round

Prior rounds (dogfood-2..4) either ran on vampiro's own workspace or triaged only by
sampling. The value proposition — *"actionable composition findings on real code with
<5% FP"* — had never been tested against a foreign corpus after the af2/51v/uah fixes.
This round runs the tool across the full ecosystem family and manually verifies
findings against source.

## Results

| Repo | Composition | Redundancy | Swallowed | Facade | Total |
|---|---:|---:|---:|---:|---:|
| wai | 145 | 38 | 0 | 0 | 183 |
| vampiro (self) | 42 | 14 | 4 | 1 | 61 |
| dont | 56 | 19 | 0 | 0 | 75 |
| espectacular | 41 | 15 | 1 | 0 | 57 |
| dulce-de-leche | 35 | 9 | 0 | 0 | 44 |
| testaruda | 32 | 4 | 0 | 0 | 36 |
| pretender | 27 | 15 | 0 | 0 | 42 |
| fotos | 15 | 2 | 0 | 0 | 17 |
| **Total** | **393** | **116** | **5** | **1** | **515** |

## Triage method

All 393 composition-breaks were bucketed by shape signature; the 66 findings whose
evidence contains no `unit` were enumerated and ~20 findings across every signature
class were manually verified against source (all 8 repos compile, so any flagged
mismatch on compiling code is an FP by construction). Robustness and modularity
findings were checked individually.

## Verdict

**The value proposition is not yet true on foreign code.**

- Composition: **393/393 FP** (sampled ~20 across all classes, zero TPs)
- Redundancy: **116/116 FP** on real code (all in vampiro's own seeded fixtures are TPs)
- Swallowed-effect: 1 on real code (espectacular `runner.rs:136`) is an FP (`?` handled it); the other 4 are vampiro's own intentional fixtures (TP)
- Facade-leak: 1, in vampiro's intentional fixture (TP)

**FP rate on real foreign code: ~100%** (target <5%).

Honest trend vs dogfood-2 (July, 543 findings, ~99.6% FP): the af2/51v/uah fixes
*did* eliminate FPs on vampiro's own codebase (dogfood-4: 0% on core code) — but that
codebase is small, single-crate-idiomatic, and constantly dogfooded. Foreign,
multi-crate, real-world Rust re-opens every inference gap. The finding: **the
composition tracer is only as good as the frontend's type-shape resolution, and the
Rust frontend gives up (`Unit`) far too often.**

## Root causes (ranked by impact, tickets filed)

| # | Root cause | Share of composition FPs | Ticket | Fix sketch |
|---|---|---:|---|---|
| R1 | `extract_shape()` maps **any non-generic named type to `Scalar(Unit)`** (`PathBuf`, custom structs, lifetime-only generics). `Result<PathBuf>` reads as `Result<unit>` and fires against anything. | 327/393 (83%) **involved a unit shape**; R1 removed the named-type-fallback subset (genuine `()` returns remain) | vampiro-224.3 | Return `Shape::Opaque` for unresolvable named types; `unify_shapes` already excludes Opaque |
| R2 | `Ok(e)` / `Some(e)` / `Err(e)` rewraps not modeled — inner shape leaks through | ~4% | vampiro-224.4 | Wrap inferred inner shape in `Parameterized{Result\|Option}` |
| R3 | Result **error parameter** compared, though `?` auto-converts errors in compiling code | ~3% | vampiro-224.5 | Compare success param only when both sides are `Result` |
| R4 | Option/Result **combinators** (`is_some_and`, `find`, `ok().flatten()`, `unwrap_or`) mis-shape slots/codomains | ~5% | vampiro-224.6 | Combinator table → output shape; unknown methods → Opaque |
| R5 | `if`/`match` **condition slots** inherit unrelated local shapes (`if is_leap(year)` → bool vs int) | ~3% | vampiro-224.7 | Condition operands expect `Scalar(Bool)` |
| R6 | Redundancy tracer groups branch shapes **per function**, not per expression — unrelated match arms compared | 116/116 redundancy | vampiro-224.8 | Expression-level grouping; ignore Opaque branches |

### Verified-FP examples (one per class)

- **R1** — `wai src/commands/pipeline/mod.rs:319`: `let hash = artifact_hash(path)?;` — `artifact_hash -> Result<String>` (correct) vs caller `Result<PathBuf>` read as `Result<unit>`. Code is fine.
- **R2** — `fotos src-tauri/src/credentials.rs:32`: `Ok(entry.get_password()?)` flagged `string` vs `Result<string>`.
- **R3** — `espectacular src/lint.rs:142-143`: three `anyhow::Result<...>` returns flagged on error-param-only differences.
- **R4** — `wai src/commands/way/hooks.rs:25`: `read_hook(...).is_some_and(...)` flagged `Option<string>` vs `bool`.
- **R5** — `dont src/events.rs:91`: `if is_leap(year)` flagged `bool` vs `int` (expected shape stolen from neighboring match arms).
- **R6** — `dont src/events.rs:91` redundancy: match arms of `days_in_month` (ints) grouped with an unrelated 6-tuple.

## What would make the value proposition true

1. **R1 is the unlock**: a ~10-line fallback change in `extract_shape()` (plus
   nested-opaque/bottom-aware `unify_shapes` extensions) removes the
   named-type-fallback share of composition noise on foreign code with
   near-zero TP cost (no compiling Rust code has a real `unit`-vs-`Result<PathBuf>`
   break — the compiler catches those). Measured: −53% composition, −42% total.
2. R2–R5 are bounded frontend inference work; together with R1 they project to
   <15% residual composition FP on this corpus.
3. R6 makes the redundancy axis usable at all (currently 100% FP on real code).
4. Re-run this exact round (same 8 repos) as the acceptance test for the epic.
   Success criterion: composition FP rate <25% post-R1, <5% after the full epic.

## Extension: specodelic + bajan (same day)

Re-ran the post-R1 binary over two more family repos. specodelic exercised the
Python frontend on real foreign code (`scripts/check_section_sync.py`) — the
cross-language claim is live, but its annotations have the R1 problem in their
own mapping (`list[str]` → Unit, vampiro-224.11).

| Repo | Total | Composition | Redundancy | Notes |
|---|---:|---:|---:|---|
| specodelic | 72 | 54 | 18 | ~25 findings are one new FP class (see 224.9) |
| bajan | 6 | 3 | 3 | all FP, mostly 224.9 |

### New root causes filed

| # | Root cause | Ticket | Evidence |
|---|---|---|---|
| R7 | **Return-boundary check fires for statement-position / let-bound calls** — compares callee result to the caller's return type even when the result is never returned. In compiling code, definitionally FP. Biggest remaining class. | vampiro-224.9 (P1) | specodelic main.rs:936 `emit_report(...); return 2;` (×25), bajan nodes.rs:204 |
| R8 | `tests/` directory files not marked as test code (is_test only detects `#[cfg(test)]`/`#[test]`) | vampiro-224.10 | specodelic tests/cli.rs:581, tests/sibling_blockers.rs:51-53 |
| R9 | Python frontend subscripted annotations (`list[str]`, `dict[K,V]`) degrade to Unit — Python's own R1 | vampiro-224.11 | check_section_sync.py:125/128 |

Note: R7 is arguably the most important finding of the whole round — the
return-boundary concept itself (callee codomain vs caller codomain) is only
sound for return-position calls; everywhere else it approximates and noise
dominates. The data-flow check (uah) is the correct instrument for everything
else.

Methodology note: "compiling code ⇒ FP by construction" holds for Rust only.
The Python-file finding above is FP by manual verification
(`problems += check(path)` with `check -> list[str]` is correct code), not by
construction.

## Interim: vampiro-s3e (unit-callee-codomain gate)

Fix: return-boundary check no longer fires when the RAW callee codomain is
`Scalar(Unit)` on a same-language Rust boundary (both edge files `.rs`);
the gate keys on the raw codomain so `?`-unwrapped `Result<T,_>` callees
with unresolved `T` still compare (the try-operator TP fixture caught this
 distinction during TDD). Cross-language edges are intentionally not gated
(rustc protects nothing there).

Measured on the original 8 repos: composition 185 → **162** (−23), total
299 → **276**. Per-repo counts in `dogfood-5-corpus.json` (`post-s3e`).
The projected −96 did not materialize: the surviving unit involvement is
**nested** unit in the caller's compound codomain (`Result<()>` callers
with statement-position calls — the classic
`fn setup() -> Result<()> { other_thing(); Ok(()) }` idiom), which is the
vampiro-224.9 class, not a raw unit callee. Fixing 224.9 next is expected
to remove most of the remainder. One seeded-fixture expectation updated
(`redundancy.expected.json`): the two REQ-7 composition-breaks there were
themselves unit-callee FPs from the hand-built graph; the seeded REQ-11
redundancy TP is unchanged.

---

## Reproduction

```bash
cargo build -p vampiro
for r in dulce-de-leche wai dont espectacular pretender testaruda vampiro fotos; do
  (cd ~/para/areas/dev/gh/charly/$r && \
   ~/para/areas/dev/gh/charly/vampiro/target/debug/vampiro check --full --mode guidance --json)
done
```
