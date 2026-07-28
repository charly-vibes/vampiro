# Change: Add Rust CLI foundation

## Why
Vampiro needs an executable, stable foundation before analysis capabilities can be integrated.

## What Changes
- Create a stable Rust Cargo workspace and reserve the specified `check` and `prove` command families; scan workflows owns CI generation and its spelling.
- Define configuration loading, a shared finding envelope (including optional `filtration_distance = sev(e)`), and exit-code contract without implementing analysis, reporting, proof, or gating behavior.

## Impact
- Affected capability: `cli-foundation`
- Future implementation: Cargo workspace, CLI/configuration crates, shared domain types
- Source requirement: REQ-4; conformance references: REQ-5, REQ-12

## Dependencies and Order
No predecessor. Its stable contracts are milestones consumed by the platform, law, scan, and lifecycle changes.
