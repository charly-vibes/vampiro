> **Status:** Active OpenSpec proposal; not implemented or deployed. The source under `openspec/changes/` is authoritative.

# Change: Add Rust analysis frontend

## Why
Vampiro needs one concrete language frontend to validate CIR extraction and enable end-to-end seam checks.

## What Changes
- Parse Rust without execution and extract full CIR including bounded argument provenance, visibility/facades, lifecycle facts, implementation clusters, proof tags, and law-runner values/generators.
- Conformance-test Rust's effect and visibility idiom tables. Later changes own findings derived from this metadata.

## Impact
- Affected capability: `rust-analysis`
- Conformance references: REQ-1–3, REQ-V1–V2, REQ-10, REQ-17, REQ-T2–T3; generic contracts remain platform/law/lifecycle-owned

## Dependencies and Order
Depends on stabilized CIR/plugin platform contracts. Its first E2E milestone unlocks core; its runner-input and lifecycle-extraction milestones independently unlock law and lifecycle contracts.
