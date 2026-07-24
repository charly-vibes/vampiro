## Context
One normalized run result must drive every presentation and policy decision.

## Goals / Non-Goals
- Goals: fast defaults, incremental completeness, visible coverage, deterministic policy/output.
- Non-goals: new analysis rules, proof semantics, or frontend parsing.

## Decisions
- Build a normalized scan result before rendering or gating; derive human, JSON, and SARIF from it.
- Hash canonical rule/location/shape input for stable deduplication.
- Cache extraction by source content plus analyzer, schema, plugin, and configuration versions; compatible unchanged files have zero extraction and telemetry reports hit/miss/invalidation reasons.
- Resolve local default as `HEAD` to a synthetic worktree including staged, unstaged, and untracked non-ignored files; resolve explicit targets from first parent or an explicit base's merge base. Detached `HEAD` follows the same rule and a parentless target uses the empty tree. Missing shallow inputs and non-Git context are operational errors carrying scope/base/target, reason, and explicit-full guidance; never silently fall back to full.
- Generate pull-request CI with the provider's head commit as the explicit target and its base ref or commit through `--base`; fetch required history, resolve both inputs and their merge base to immutable commit IDs before analysis, and use the same operational error without scope broadening when resolution or fetch fails.
- Define `guidance`, `tiered`, and `gate`; `tiered` reports configured tiers while only `gate` blocks at threshold.
- Gate on configured severity unless project configuration provides a validated `filtration_distance` mapping.

## Risks / Trade-offs
- Git rename/diff ambiguity can miss seams; report scope metadata and permit explicit full scans.
- Cache invalidation errors threaten correctness; version all inputs and test cold/warm equivalence.

## Open Questions
- Confirm supported CI providers and exact tiered-mode policy configuration during implementation.
