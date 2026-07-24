# Change: Add core seam analysis

## Why
CIR must be checked consistently across languages on the four finding axes, including redundancy as robustness.

## What Changes
- Add structural unification, visibility/facade analysis, recursive effect handling, ancestor-throws reachability, and robustness-axis redundancy reconciliation.
- Preserve opaque-shape eligibility for non-composition checks and distinguish source findings from plugin diagnostics.

## Impact
- Affected capability: `seam-analysis`
- Source requirements: REQ-7–9, REQ-11, REQ-23, REQ-25, REQ-V3–V4, REQ-V7, REQ-C4–C5, REQ-C7

## Dependencies and Order
Depends on stable CIR/platform contracts and the first Rust frontend milestone. Scan depends on this core contract.
