# Verification: Section 2 — Boundary-Leak Tracer

**Date:** 2026-07-28
**Ticket:** vampiro-0vb.10.3
**Spec:** `openspec/changes/add-trust-boundary-analysis/tasks.md#2`
**Decision record:** `docs/decisions/trust-boundary-contract.md`

## Summary

Delivered the language-neutral boundary-leak analyzer. Flags untrusted data flowing into nodes that are not themselves trust-boundary sources. Smart constructors are NOT recognized without explicit configuration — any edge carrying `Untrusted` trust provenance into an interior node is a boundary leak.

## Checklist

| Item | Status | Details |
|------|--------|---------|
| 2.1 | ✓ | 7 CIR-level tests covering: untrusted→interior leak (positive), boundary-source self-loop (negative), unknown provenance (negative), forwarding node (positive), multiple edges (positive), trusted edge (negative), boundary-source identification logic. |
| 2.2 | ✓ | `BoundaryLeakAnalyzer` in `vampiro-seam-analysis` — emits `REQ-B3` / `HIGH` / `robustness` / `boundary-leak` finding per violating edge with source, edge_id, and target evidence. |
| 2.3 | ✓ | 7 tests pass (269 workspace tests, 0 failed). |

## Implementation

### Boundary-leak analyzer — `crates/vampiro-seam-analysis/src/boundary_leak.rs`

| Feature | Implementation |
|---------|---------------|
| `BoundaryLeakAnalyzer::analyze` | Scans edges with `trust_provenance: Untrusted`. Skips edges whose target is a trust-boundary source (node with untrusted output but no incoming untrusted edges from other nodes, excluding self-loops). Emits `REQ-B3` finding for all other edges. |
| `identify_boundary_sources` | Nodes with `Untrusted` output and no incoming `Untrusted` edges from other nodes (self-loops excluded). |
| `identify_smart_constructors` | (Available but unused without explicit config) Nodes with `Trusted` output that receive `Untrusted` input. |

### Evidence contract

| Field | Value |
|-------|-------|
| `rule` | `REQ-B3` |
| `axis` | `robustness` |
| `severity` | `high` (default; REQ-4 table) |
| `classification` | `boundary-leak` |
| `evidence.source` | Source node stable identity |
| `evidence.source_name` | Source node display name |
| `evidence.edge_id` | Edge stable identity |
| `evidence.target` | Target node stable identity |
| `evidence.target_name` | Target node display name |

## Test scenarios

| Test | Nodes | Edge | Expected |
|------|-------|------|----------|
| Untrusted → interior (trusted output) | src(Untrusted) → proc(Trusted) | Untrusted | 1 finding |
| Untrusted → boundary source (self-loop) | src(Untrusted) → src(Untrusted) | Untrusted | 0 findings |
| Unknown → interior | src(Unknown) → proc(Trusted) | Unknown | 0 findings |
| Untrusted → forwarding node | src(Untrusted) → fwd(Untrusted) | Untrusted | 1 finding |
| Two sources → one forwarding node | src1, src2(Untrusted) → fwd(Untrusted) | 2× Untrusted | 2 findings |
| Trusted → trusted | src(Trusted) → dest(Trusted) | Trusted | 0 findings |
| Boundary source identification | a(Untrusted) → b(Untrusted) | Untrusted | a=source, b≠source |

## Passing command output

```
$ cargo test -p vampiro-seam-analysis
test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test --workspace
(269 tests pass across all crates, 0 failed)

$ cargo fmt --all --check
(clean)

$ cargo clippy --workspace --all-targets -- -D warnings
(no warnings)

$ openspec validate add-trust-boundary-analysis --strict
Change 'add-trust-boundary-analysis' is valid
```