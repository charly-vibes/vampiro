# Dogfooding Run 5: the dulce-de-leche ecosystem (foreign-code value-proposition check)

**Date:** 2026-09-30
**Tickets:** vampiro-224 (epic) + vampiro-224.3 … vampiro-224.8
**Pipeline:** `vampiro check --full --mode guidance --json` (vampiro 0.4.0, all 8 frontends, data-flow edges, is_test filtering)
**Scope:** all 8 Rust repos of the dulce-de-leche family: `wai`, `dont`, `dulce-de-leche`, `espectacular`, `pretender`, `testaruda`, `vampiro`, `fotos` (fabbro excluded — Go)

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
| R1 | `extract_shape()` maps **any non-generic named type to `Scalar(Unit)`** (`PathBuf`, custom structs, lifetime-only generics). `Result<PathBuf>` reads as `Result<unit>` and fires against anything. | **~83%** (327/393) | vampiro-224.3 | Return `Shape::Opaque` for unresolvable named types; `unify_shapes` already excludes Opaque |
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

1. **R1 is the unlock**: a ~5-line change in `extract_shape()` removes ~83% of
   composition noise on foreign code with near-zero TP cost (no compiling Rust code
   has a real `unit`-vs-`Result<PathBuf>` break — the compiler catches those).
2. R2–R5 are bounded frontend inference work; together with R1 they project to
   <15% residual composition FP on this corpus.
3. R6 makes the redundancy axis usable at all (currently 100% FP on real code).
4. Re-run this exact round (same 8 repos) as the acceptance test for the epic.
   Success criterion: composition FP rate <25% post-R1, <5% after the full epic.

## Reproduction

```bash
cargo build -p vampiro
for r in dulce-de-leche wai dont espectacular pretender testaruda vampiro fotos; do
  (cd ~/para/areas/dev/gh/charly/$r && \
   ~/para/areas/dev/gh/charly/vampiro/target/debug/vampiro check --full --mode guidance --json)
done
```
