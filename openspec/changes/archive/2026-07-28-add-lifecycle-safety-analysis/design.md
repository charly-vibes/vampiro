## Context
Lifecycle checks combine persisted history, frontend facts, graph analysis, and optional law evidence.

## Goals / Non-Goals
- Goals: conservative identity, migration-aware breakage, honest retry classification, total release checking.
- Non-goals: N+1 detection, lock ordering, source execution, or heuristic renames treated as certainty.

## Decisions
- Before persistence implementation, select and document snapshot storage, schema/version, retention, baseline override, and migration declaration syntax.
- Default to the nearest persisted first-parent ancestor; an explicit override must exist and be an ancestor. First snapshot establishes history without a breaking finding; missing/non-ancestor explicit baselines are errors. Multiple snapshots at a revision are selected deterministically by the documented schema key.
- Treat ambiguous historical identity as coverage information, not add/remove events.
- Derive retry idempotency from a versioned write-shape table with `unknown`; cross-reference, never merge, robustness and optionality findings.
- Give each acquisition a unique obligation/resource identity. Match exactly one release to it; a duplicate release cannot discharge another obligation and therefore contributes to REQ-T7 when an obligation remains pending. Preserve identity across explicit transfer and diagnose unknown aliases specifically as `identity:unknown`, never as safety.

## Risks / Trade-offs
- Snapshot storage and identity schemas can lock in history; version schemas and retain migration readers.
- Control-flow approximation can produce uncertainty; preserve unknown paths and report coverage rather than asserting safety.

## Decision Gate
- A HITL decision ticket SHALL confirm snapshot storage/schema/version,
  retention, baseline and migration syntax, migration-reader policy, and initial
  write/resource idiom sets before persistence or classifier implementation.
