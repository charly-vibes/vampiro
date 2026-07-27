> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

# Change: Add scan, gating, and reporting workflows

## Why
Analysis becomes operationally useful only when scoped, serialized, deduplicated, gated, and integrated into CI predictably.

## What Changes
- Add strict Git diff defaults, explicit incremental full scope, unsupported-language coverage, policy modes, stable deduplication, severity/filtration evidence parity, CI generation, and a <9-second 50-edge benchmark profiled by hardware, OS, repository size/state, cache state, plugin/tool/config/platform versions, and measurement method.

## Impact
- Affected capability: `scan-workflows`
- Source requirements: REQ-5, REQ-13–15, REQ-19–20, REQ-24, REQ-27–28

## Dependencies and Order
Depends on the core finding contract and integrates with stable CLI command/exit contracts; it does not serialize law or lifecycle work.
