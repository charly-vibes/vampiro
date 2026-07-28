# Verification: Section 1 — Trust-Provenance Contract Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.10.2
**Spec:** `openspec/changes/add-trust-boundary-analysis/tasks.md#1`
**Decision record:** `docs/decisions/trust-boundary-contract.md`

## Summary

Delivered the CIR extension for trust-provenance classification as the first slice of the trust-boundary analysis epic. Introduces the `TrustProvenance` enum (`Untrusted`/`Trusted`/`Unknown`) in `vampiro-cir`, adds it to `CirNode` and `CirEdge` with `#[serde(default)]` for backward compatibility, and implements the join/transfer semantics (`Untrusted > Unknown > Trusted`).

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 1.1 | ✓ | TrustProvenance enum with serde round-trip tests, join/transfer truth table with 9 exhaustive cases and commutativity proof, Display impl, predicate methods, and backward-compatible deserialization of old graphs (no `trust_provenance` field). |
| 1.2 | ✓ | `TrustProvenance` added to `CirNode.trust_provenance` (node output) and `CirEdge.trust_provenance` (edge argument slot) with `#[serde(default)]` defaulting to `Trusted`. Join order implemented as `TrustProvenance::join`. |
| 1.3 | ✓ | 39 CIR tests pass (including 8 new trust-provenance tests). Serialized-schema round-trip verified. |

## Implementation

### CIR extension — `crates/vampiro-cir/src/provenance.rs`

| Feature | Implementation |
|---------|---------------|
| `TrustProvenance` enum | Three variants: `Untrusted`, `Trusted`, `Unknown`. Serde serializes as kebab-case (`"untrusted"`/`"trusted"`/`"unknown"`). `Default` is `Trusted`. |
| `TrustProvenance::join` | Binary join with `Untrusted > Unknown > Trusted` order. Any untrusted contributor → `Untrusted`; else any unknown → `Unknown`; else `Trusted`. Commutative. |
| `is_untrusted/is_trusted/is_unknown` | Predicate accessors for convenience. |
| `Display` | Render as lowercase strings. |

### CIR graph — `crates/vampiro-cir/src/cir.rs`

| Addition | Field | Default |
|----------|-------|---------|
| `CirNode.trust_provenance` | Classifies the node's output value | `Trusted` (via `#[serde(default)]`) |
| `CirEdge.trust_provenance` | Classifies the argument flowing through this edge | `Trusted` (via `#[serde(default)]`) |

Both fields use `#[serde(default)]` so existing serialized CIR graphs (produced before this change) deserialize without modification — the field defaults to `Trusted`, which is the correct same-origin default.

## Fixtures

Located at `tests/fixtures/add-trust-boundary-analysis/1/` (future E2E Rust fixtures to be added when trust-source idiom recognition is implemented in the Rust frontend).

## Trust-provenance join truth table

| Left | Right | Result | Rationale |
|------|-------|--------|-----------|
| `Untrusted` | `Untrusted` | `Untrusted` | Both untrusted |
| `Untrusted` | `Trusted` | `Untrusted` | Untrusted dominates |
| `Untrusted` | `Unknown` | `Untrusted` | Untrusted dominates |
| `Trusted` | `Untrusted` | `Untrusted` | Untrusted dominates |
| `Trusted` | `Trusted` | `Trusted` | Both trusted |
| `Trusted` | `Unknown` | `Unknown` | Unknown dominates trusted |
| `Unknown` | `Untrusted` | `Untrusted` | Untrusted dominates |
| `Unknown` | `Trusted` | `Unknown` | Unknown dominates trusted |
| `Unknown` | `Unknown` | `Unknown` | Both unknown |

The join is commutative: `a.join(b) == b.join(a)` for all `a, b`.

## Contract versions

| Contract | Version | Location |
|----------|---------|----------|
| CIR schema | `0.1.0` | `vampiro-cir` crate; `CirGraph.version` |
| Trust provenance | `0.1.0` (in-progress; published as part of trust-boundary analysis) | `vampiro-cir::provenance::TrustProvenance` |

## Passing command output

```
$ cargo test -p vampiro-cir
test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --workspace
(262 tests pass across all crates, 0 failed)

$ cargo fmt --all --check
(clean)

$ cargo clippy --workspace --all-targets -- -D warnings
(no warnings)

$ openspec validate add-trust-boundary-analysis --strict
Change 'add-trust-boundary-analysis' is valid
```