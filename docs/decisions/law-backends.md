# Law Backends Decision Gate

**Date:** 2026-07-29  
**Approver:** charly vibes  
**Status:** Approved  

## Rust Property-Testing Crate

**Chosen:** `proptest` 1.x

**Alternatives considered:**
- `quickcheck` — rejected: no deterministic seed support, no explicit generator config struct, less control over test case count
- `bolero` — rejected: fuzz-focused, heavier dependency, less mature ecosystem
- Custom property tester — rejected: unnecessary engineering when off-the-shelf works

**Rationale:** proptest supports deterministic generation via `TestRunner::run()` with explicit `Config { cases, seed, .. }`. This enables reproducible property tests and seed-gated evidence, matching REQ-18 (generated values as evidence). The seed 42 is the canonical default.

**Supported version:** 1.5+ (inclusive). CI validates against proptest 1.5 via `Cargo.lock` compatibility.

## Prover Backends

### Lean 4

**Input format:** `.lean` file with a `theorem` declaration. The adapter generates a stub theorem that the prover checks.

**Process interface:** `lean <file>` — subprocess, stdout for success, stderr for errors.

**Security boundary:** Subprocess isolation. The input file is written to a temp directory cleaned up after execution. No network access. Max input size: 10 KB (hard-coded cap in adapter).

### Dafny 4+

**Input format:** `.dfy` file with a `method` declaration and postcondition.

**Process interface:** `dafny verify <file>` — subprocess, stdout for status, stderr for errors.

**Security boundary:** Same as Lean (subprocess, temp dir, no network).

### TLA+ (TLC)

**Input format:** `.tla` module file with an `Invariant` formula.

**Process interface:** `tlc <file>` — subprocess, stdout for success/failure.

**Security boundary:** Same as Lean/Dafny.

## Timeout and Resource Policy

| Setting | Value | Notes |
|---------|-------|-------|
| Default timeout | 30s | Configurable per obligation via `Obligation` metadata |
| Hard deadline | 300s | Enforced by the CLI, not the adapter |
| Max input size | 10 KB | Prevents DoS via oversized generated code |
| Temp dir max files | 1 per adapter run | Cleaned up on drop |

**Known limitation:** The current adapter implementations accept a `timeout` parameter but do not enforce a hard subprocess deadline via OS mechanisms (e.g., `setrlimit` or `timeout` wrapper). A hanging prover may hang the caller indefinitely. This is tracked as a future improvement.

## Rejected Alternatives

1. **Inline theorem generation (no subprocess):** Rejected — linking Lean/Dafny as libraries would be infeasible (heavy deps, version conflicts across three provers).
2. **HTTP/REST API wrappers:** Rejected — adds unnecessary infrastructure; subprocess is the simplest process boundary.
3. **Single `prove` binary wrapping all three:** Rejected — each prover has different CLI flags and output formats; separate per-prover adapters are cleaner.
4. **`quickcheck` over `proptest`:** Rejected — see Rust Property-Testing section above.

## Golden-Translation Test Examples

The adapter `generate_*` methods are tested via golden-style assertions in `prover.rs`:

- Lean: `lean_generates_valid_theorem` — asserts `theorem` keyword, theory+member ID, law text
- Dafny: `dafny_generates_valid_method` — asserts `method` keyword, theory+member ID
- TLA+: `tla_plus_generates_valid_spec` — asserts `MODULE` keyword, theory+member ID

All three tests pass with 0 fixtures needed (the assertions are on generated text, not compiled output).

## Immutable Review Reference

This decision is recorded in git as `docs/decisions/law-backends.md`.  
Commit: `55f44dd`.  
Any future changes to backends, versions, or process boundaries require a new decision document and HITL approval.