# Law Backends Decision

> Property-testing crate and prover adapter boundaries for law-verification.

**Approver:** charly vibes
**Review reference:** bd issue vampiro-0vb.6.1 — Property and prover boundary decision gate
**Date:** 2026-07-28

---

## 1. Rust property-testing crate

### Alternatives considered

| Crate | Shrink | Generators | Deterministic | Decision |
|-------|--------|------------|---------------|----------|
| **`proptest`** | Auto-shrink | `prop_*` strategies | `TestRunner` with seed | ✅ Selected |
| `quickcheck` | Custom `Arbitrary` | `Arbitrary` trait | `StdThreadGen` seed | ❌ Rejected |
| `bolero` | Custom | Fuzz-like | Yes | ❌ Rejected |

**Decision:** `proptest`.

**Rationale:**
- Auto-shrink produces minimal counterexamples without manual shrink impls.
- `TestRunner` with explicit `Config` (seed, cases) makes runs deterministic.
- Largest ecosystem, Rust org maintained, maps cleanly to law obligations.

**Trigger to revisit:** Benchmark shows `proptest` as bottleneck at 1M+ generator iterations.

---

## 2. Obligation IR format

**Decision:** Native Rust types in a dedicated `obligation` module.

**Rationale:**
- No serialization surface between obligation and runner.
- `Obligation` (theory + cluster + generator config) and `Evidence` (passed/failed/inconclusive/error + trace) versioned by crate version.

---

## 3. Prover input formats

| Prover | Input | Integration |
|--------|-------|-------------|
| **Lean** | `.lean` theorem | Write file → spawn `lean` → parse stdout |
| **Dafny** | `.dfy` method + ensures | Translate obligation → `dafny verify` → parse |
| **TLA+** | `.tla` invariant | Translate → `tlc` → parse output |
| **None** (default) | N/A | Property testing only |

All optional, behind `vampiro prove --prover <name>`. Never invoked during `check`.

---

## 4. Prover process boundary

| Aspect | Decision |
|--------|----------|
| Execution | Subprocess with timeout. No C-FFI. |
| Timeout | Configurable, default 30s, hard cap 300s |
| Malformed | Exit code != 0 → `Status::ProverUnavailable` |
| Missing tool | Not in `$PATH` → `Status::ProverUnavailable` |
| Security | No sandbox. Opt-in command only. |

---

## 5. Evidence statuses

| Status | Meaning |
|--------|---------|
| `Proved` | Prover confirmed the obligation |
| `Disproved` | Prover found a counterexample |
| `Timeout` | Did not complete within timeout |
| `ProverUnavailable` | Tool not found or errored |

---

## 6. Rejected alternatives

| Alternative | Reason |
|-------------|--------|
| `quickcheck` | No auto-shrink, weaker determinism |
| `bolero` | Fuzz harness, not property testing |
| Embedded prover (FFI) | Subprocess simpler, safer, more portable |
| Prover during `check` | Violates REQ-10 (no source execution during static check) |
| JSON obligation schema | Over-engineering for Rust-native runner |

---

## 7. Scope and compatibility

- **Property crate:** `proptest` 1.x
- **Provers:** Optional, feature-gated modules
- **Immutability:** Valid until second crate or prover feature changes the process boundary