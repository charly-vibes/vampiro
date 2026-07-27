> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

# Change: Add lifecycle safety analysis

## Why
Seam safety also spans versions, retries, and resource lifetimes, which require historical and control-flow-aware checks beyond core seam analysis.

## What Changes
- Add nearest-first-parent facade baselines with explicit override, migration-aware evolution checks, deterministic identity handling, retry idempotency law linkage, and identity-based acquire/release analysis.

## Impact
- Affected capability: `lifecycle-safety`
- Source requirements: REQ-T1–T9

## Dependencies and Order
Depends on CIR/platform contracts, core finding contracts, the law-verification idempotency-evidence contract, and Rust lifecycle extraction. It does not depend on completed scan; additional frontends may consume its stable contracts independently.
